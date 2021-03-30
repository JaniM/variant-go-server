use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::{db, server};
use shared::game;
use shared::game::clock::Millisecond;
use shared::message;

// TODO: add room timeout

///////////////////////////////////////////////////////////////////////////////
//                               Actor messages                              //
///////////////////////////////////////////////////////////////////////////////

// Output /////////////////////////////////////////////////////////////////////

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    // TODO: Use a proper struct, not magic tuples
    GameStatus {
        room_id: u32,
        owner: u64,
        members: Vec<u64>,
        view: game::GameView,
    },
    BoardAt {
        room_id: u32,
        view: game::GameHistory,
    },
    SGF {
        room_id: u32,
        sgf: String,
    },
}

// Actions ////////////////////////////////////////////////////////////////////

pub struct GameAction {
    pub id: usize,
    pub action: message::GameAction,
}

impl actix::Message for GameAction {
    type Result = Result<(), message::Error>;
}

pub struct GameActionAsUser {
    pub user_id: u64,
    pub action: message::GameAction,
}

impl actix::Message for GameActionAsUser {
    type Result = Result<(), message::Error>;
}

// User lifecycle /////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    pub session_id: usize,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub session_id: usize,
    pub user_id: u64,
    pub addr: Recipient<Message>,
}

// Control ////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "()")]
pub struct Unload;

pub struct GetAdminView;

impl actix::Message for GetAdminView {
    type Result = Result<game::GameView, message::Error>;
}

///////////////////////////////////////////////////////////////////////////////
//                                   Actor                                   //
///////////////////////////////////////////////////////////////////////////////

pub struct GameRoom {
    pub room_id: u32,
    pub owner: Option<u64>,
    pub sessions: HashMap<usize, (u64, Recipient<Message>)>,
    pub users: HashSet<u64>,
    pub name: String,
    pub last_action: Instant,
    pub game: game::Game,
    pub db: Addr<db::DbActor>,
    pub server: Addr<server::GameServer>,

    /// Kicked players are not visible to other users in the game and can not
    /// hold seats. They can still follow the game.
    pub kicked_players: HashSet<u64>,
}

impl GameRoom {
    fn send_room_messages(&self, mut create_msg: impl FnMut(u64) -> Message) {
        for (user_id, addr) in self.sessions.values() {
            let _ = addr.do_send(create_msg(*user_id));
        }
    }

    fn view_for_user(&self, user_id: u64) -> Message {
        Message::GameStatus {
            room_id: self.room_id,
            owner: self.owner.unwrap_or(0),
            members: self
                .users
                .iter()
                .copied()
                .filter(|id| !self.kicked_players.contains(id))
                .collect(),
            view: self.game.get_view(user_id),
        }
    }

    fn make_action(
        &mut self,
        user_id: u64,
        action: message::GameAction,
        addr: Option<Recipient<Message>>,
    ) -> Result<(), message::Error> {
        use message::Error;

        let current_time = Millisecond(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as i128,
        );

        self.last_action = Instant::now();
        let res = match action {
            message::GameAction::Place(x, y) => self
                .game
                .make_action(user_id, game::ActionKind::Place(x, y), current_time)
                .map_err(Into::into),
            message::GameAction::Pass => self
                .game
                .make_action(user_id, game::ActionKind::Pass, current_time)
                .map_err(Into::into),
            message::GameAction::Cancel => self
                .game
                .make_action(user_id, game::ActionKind::Cancel, current_time)
                .map_err(Into::into),
            message::GameAction::Resign => self
                .game
                .make_action(user_id, game::ActionKind::Resign, current_time)
                .map_err(Into::into),
            message::GameAction::TakeSeat(seat_id) => {
                if self.kicked_players.contains(&user_id) {
                    return Err(Error::other("Kicked from game"));
                }
                self.game
                    .take_seat(user_id, seat_id as _)
                    .map_err(Into::into)
            }
            message::GameAction::LeaveSeat(seat_id) => self
                .game
                .leave_seat(user_id, seat_id as _)
                .map_err(Into::into),
            message::GameAction::BoardAt(start, end) => {
                let addr = addr.expect("Address needed to get board position");
                if start > end {
                    return Ok(());
                }
                // Prevent asking for a ridiculous amount.
                if end as usize > self.game.shared.board_history.len() + 20 {
                    return Ok(());
                }
                for turn in (start..=end).rev() {
                    let view = self.game.get_view_at(user_id, turn);
                    if let Some(view) = view {
                        let _ = addr.do_send(Message::BoardAt {
                            room_id: self.room_id,
                            view,
                        });
                    }
                }
                return Ok(());
            }
            message::GameAction::RequestSGF => {
                let addr = addr.expect("Address needed to get SGF");
                let game_done = matches!(self.game.state, game::GameState::Done(_));
                if !game_done {
                    return Err(Error::other("Game not finished"));
                }
                let sgf = game::export::sgf_export(&self.game);
                let _ = addr.do_send(Message::SGF {
                    room_id: self.room_id,
                    sgf,
                });
                return Ok(());
            }
            message::GameAction::KickPlayer(kick_player_id) => {
                if self.owner != Some(user_id) {
                    return Err(Error::other("Not room owner"));
                }

                for (idx, seat) in self.game.shared.seats.clone().into_iter().enumerate() {
                    if seat.player == Some(kick_player_id) {
                        let _ = self.game.leave_seat(kick_player_id, idx);
                    }
                }
                if self.users.contains(&kick_player_id) {
                    self.kicked_players.insert(kick_player_id);
                }
                Ok(())
            }
        };

        if let Err(err) = res {
            return Err(Error::Game {
                room_id: self.room_id,
                error: err,
            });
        }

        self.db.do_send(db::StoreGame {
            id: Some(self.room_id as _),
            name: self.name.clone(),
            replay: Some(self.game.dump()),
            owner: self.owner,
        });

        self.send_room_messages(|user_id| self.view_for_user(user_id));

        Ok(())
    }
}

impl Actor for GameRoom {
    type Context = Context<Self>;

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        println!("Room {} stopping!", self.room_id);

        Running::Stop
    }
}

impl Handler<Leave> for GameRoom {
    type Result = ();

    fn handle(&mut self, msg: Leave, _ctx: &mut Self::Context) -> Self::Result {
        let Leave { session_id } = msg;

        if let Some((user_id, _addr)) = self.sessions.remove(&session_id) {
            let sessions = &self.sessions;
            if !sessions.values().any(|(uid, _addr)| *uid == user_id) {
                self.users.remove(&user_id);
                self.send_room_messages(|user_id| self.view_for_user(user_id));
            }
        }
    }
}

impl Handler<Join> for GameRoom {
    type Result = ();

    fn handle(&mut self, msg: Join, _ctx: &mut Self::Context) -> Self::Result {
        let Join {
            session_id,
            user_id,
            addr,
        } = msg;

        self.sessions.insert(session_id, (user_id, addr));
        self.users.insert(user_id);
        self.send_room_messages(|user_id| self.view_for_user(user_id));

        // TODO: Announce profile to room members

        // Broadcast the profile of each seatholder
        // .. this is not great
        for seat in &self.game.shared.seats {
            if let Some(user_id) = seat.player {
                self.server.do_send(server::QueryProfile { user_id });
            }
        }
    }
}

impl Handler<GameAction> for GameRoom {
    type Result = MessageResult<GameAction>;

    fn handle(&mut self, msg: GameAction, _: &mut Context<Self>) -> MessageResult<GameAction> {
        use message::Error;

        let GameAction { id, action } = msg;

        // TODO: PUZZLE Add background timer for clock

        let &(user_id, ref addr) = match self.sessions.get(&id) {
            Some(x) => x,
            None => return MessageResult(Err(Error::other("No session"))),
        };
        let addr = addr.clone();

        MessageResult(self.make_action(user_id, action, Some(addr)))
    }
}

impl Handler<GameActionAsUser> for GameRoom {
    type Result = MessageResult<GameActionAsUser>;

    fn handle(
        &mut self,
        msg: GameActionAsUser,
        _: &mut Context<Self>,
    ) -> MessageResult<GameActionAsUser> {
        let GameActionAsUser { user_id, action } = msg;

        MessageResult(self.make_action(user_id, action, None))
    }
}

impl Handler<Unload> for GameRoom {
    type Result = ();

    fn handle(&mut self, _: Unload, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}

impl Handler<GetAdminView> for GameRoom {
    type Result = <GetAdminView as actix::Message>::Result;

    fn handle(&mut self, _: GetAdminView, _ctx: &mut Self::Context) -> Self::Result {
        Ok(self.game.get_view(0))
    }
}
