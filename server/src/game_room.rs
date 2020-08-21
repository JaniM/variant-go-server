use actix::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use crate::db;
use crate::game;
use crate::message;

// TODO: add room timeout

macro_rules! catch {
    ($($code:tt)+) => {
        (|| Some({ $($code)+ }))()
    };
}

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    // TODO: Use a proper struct, not magic tuples
    GameStatus {
        room_id: u32,
        members: Vec<u64>,
        view: game::GameView,
    },
    BoardAt {
        room_id: u32,
        view: game::GameHistory,
    },
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GameAction {
    pub id: usize,
    pub action: message::GameAction,
}

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

pub struct GameRoom {
    pub room_id: u32,
    pub sessions: HashMap<usize, (u64, Recipient<Message>)>,
    pub users: HashSet<u64>,
    pub name: String,
    pub last_action: Instant,
    pub game: game::Game,
    pub db: Addr<db::DbActor>,
}

impl GameRoom {
    fn send_room_message(&self, msg: Message) {
        for (_, addr) in self.sessions.values() {
            let _ = addr.do_send(msg.clone());
        }
    }
}

impl Actor for GameRoom {
    type Context = Context<Self>;

    fn stopping(&mut self, ctx: &mut Self::Context) -> Running {
        println!("Room {} stopping!", self.room_id);

        Running::Stop
    }
}

impl Handler<Leave> for GameRoom {
    type Result = ();

    fn handle(&mut self, msg: Leave, ctx: &mut Self::Context) -> Self::Result {
        let Leave { session_id } = msg;

        if let Some((user_id, addr)) = self.sessions.remove(&session_id) {
            let sessions = &self.sessions;
            if !sessions.values().any(|(uid, _addr)| *uid == user_id) {
                self.users.remove(&user_id);
                let msg = Message::GameStatus {
                    room_id: self.room_id,
                    members: self.users.iter().copied().collect(),
                    view: self.game.get_view(),
                };
                self.send_room_message(msg);
            }
        }
    }
}

impl Handler<Join> for GameRoom {
    type Result = ();

    fn handle(&mut self, msg: Join, ctx: &mut Self::Context) -> Self::Result {
        let Join {
            session_id,
            user_id,
            addr,
        } = msg;

        self.sessions.insert(session_id, (user_id, addr));
        self.users.insert(user_id);
        let msg = Message::GameStatus {
            room_id: self.room_id,
            members: self.users.iter().copied().collect(),
            view: self.game.get_view(),
        };
        self.send_room_message(msg);

        // TODO: Announce profile to room members
    }
}

impl Handler<GameAction> for GameRoom {
    type Result = ();

    fn handle(&mut self, msg: GameAction, _: &mut Context<Self>) {
        let GameAction { id, action } = msg;

        let &(user_id, ref addr) = match self.sessions.get(&id) {
            Some(x) => x,
            None => return,
        };

        self.last_action = Instant::now();
        // TODO: Handle errors in game actions - currently they fail quietly
        match action {
            message::GameAction::Place(x, y) => {
                let res = self
                    .game
                    .make_action(user_id, game::ActionKind::Place(x, y));
                if res.is_err() {
                    return;
                }
            }
            message::GameAction::Pass => {
                let res = self.game.make_action(user_id, game::ActionKind::Pass);
                if res.is_err() {
                    return;
                }
            }
            message::GameAction::Cancel => {
                let res = self.game.make_action(user_id, game::ActionKind::Cancel);
                if res.is_err() {
                    return;
                }
            }
            message::GameAction::TakeSeat(seat_id) => {
                let res = self.game.take_seat(user_id, seat_id as _);
                if res.is_err() {
                    return;
                }
            }
            message::GameAction::LeaveSeat(seat_id) => {
                let res = self.game.leave_seat(user_id, seat_id as _);
                if res.is_err() {
                    return;
                }
            }
            message::GameAction::BoardAt(start, end) => {
                if start > end {
                    return;
                }
                for turn in start..=end {
                    let view = self.game.get_view_at(turn);
                    if let Some(view) = view {
                        let _ = addr.do_send(Message::BoardAt {
                            room_id: self.room_id,
                            view,
                        });
                    }
                }
                return;
            }
        }

        self.db.do_send(db::StoreGame {
            id: Some(self.room_id as _),
            name: self.name.clone(),
            replay: Some(self.game.dump()),
        });

        self.send_room_message(Message::GameStatus {
            room_id: self.room_id,
            members: self.users.iter().copied().collect(),
            view: self.game.get_view(),
        });
    }
}
