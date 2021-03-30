use actix::prelude::*;
use rand::prelude::*;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::db;
use crate::game_room::{self, GameRoom};
use shared::game;
use shared::message::{self, AdminAction};

macro_rules! catch {
    ($($code:tt)+) => {
        (|| Some({ $($code)+ }))()
    };
}

///////////////////////////////////////////////////////////////////////////////
//                               Actor messages                              //
///////////////////////////////////////////////////////////////////////////////

// Output /////////////////////////////////////////////////////////////////////

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    // TODO: Use a proper struct, not magic tuples
    AnnounceRoom(u32, String),
    CloseRoom(u32),
    Identify(Profile),
    UpdateProfile(Profile),
}

// Client lifetime ////////////////////////////////////////////////////////////

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
    pub game_addr: Recipient<game_room::Message>,
}

/// Session is disconnected
#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: usize,
}

// Rooms //////////////////////////////////////////////////////////////////////

/// List of available rooms
pub struct ListRooms;

impl actix::Message for ListRooms {
    // TODO: Use a proper struct, not magic tuples
    type Result = Vec<(u32, String)>;
}

/// Join room
pub struct Join {
    /// Client id
    pub id: usize,
    pub room_id: u32,
    pub leave_previous: bool,
}

impl actix::Message for Join {
    type Result = Result<Addr<GameRoom>, ()>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct LeaveRoom {
    /// Client id
    pub id: usize,
    /// Null for all
    pub room_id: Option<u32>,
}

/// Create room, announce to clients
pub struct CreateRoom {
    /// Client id
    pub id: usize,
    pub room: message::StartGame,
    pub leave_previous: bool,
}

impl actix::Message for CreateRoom {
    type Result = Result<(u32, Option<Addr<GameRoom>>), message::Error>;
}

// User management ////////////////////////////////////////////////////////////

pub struct IdentifyAs {
    pub id: usize,
    pub token: Option<String>,
    pub nick: Option<String>,
}

impl actix::Message for IdentifyAs {
    type Result = Result<Profile, message::Error>;
}

pub struct QueryProfile {
    pub user_id: u64,
}

impl actix::Message for QueryProfile {
    type Result = Result<Profile, ()>;
}

pub struct GetAdminView {
    pub room_id: u32,
}

impl actix::Message for GetAdminView {
    type Result = Result<game::GameView, ()>;
}

// Admin //////////////////////////////////////////////////////////////////////

#[derive(Message)]
#[rtype(result = "()")]
pub struct AdminMessage {
    pub client_id: usize,
    pub action: AdminAction,
}

///////////////////////////////////////////////////////////////////////////////
//                                    Data                                   //
///////////////////////////////////////////////////////////////////////////////

#[derive(Clone)]
pub struct Profile {
    pub user_id: u64,
    pub token: Uuid,
    pub nick: Option<String>,
    pub last_game_time: Option<Instant>,
    pub is_admin: bool,
}

pub struct Session {
    pub user_id: Option<u64>,
    pub client: Recipient<Message>,
    pub game_client: Recipient<game_room::Message>,
    pub room_ids: Vec<u32>,
}

#[derive(Clone)]
pub struct Room {
    pub addr: Addr<GameRoom>,
    pub name: String,
}

///////////////////////////////////////////////////////////////////////////////
//                                   Actor                                   //
///////////////////////////////////////////////////////////////////////////////

/// `GameServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct GameServer {
    sessions: HashMap<usize, Session>,
    sessions_by_user: HashMap<u64, HashSet<usize>>,
    profiles: HashMap<u64, Profile>,
    rooms: HashMap<u32, Room>,
    rng: ThreadRng,
    admin_token: Uuid,
    db: Addr<db::DbActor>,
}

impl Default for GameServer {
    fn default() -> GameServer {
        dotenv::dotenv().ok();

        let rooms = HashMap::new();
        let db = SyncArbiter::start(8, db::DbActor::default);
        let admin_token = std::env::var("ADMIN_TOKEN")
            .map_err(|_| ())
            .and_then(|x| Uuid::parse_str(&x).map_err(|_| ()))
            .unwrap_or_else(|_| Uuid::from_bytes(rand::thread_rng().gen()));

        println!("Admin token: {:?}", admin_token);

        GameServer {
            sessions: HashMap::new(),
            sessions_by_user: HashMap::new(),
            profiles: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
            admin_token,
            db,
        }
    }
}

impl GameServer {
    /// Send message to all users
    fn send_global_message(&self, message: Message) {
        for session in self.sessions.values() {
            let _ = session.client.do_send(message.clone());
        }
    }

    fn send_message(&self, session_id: usize, message: Message) {
        let session = self.sessions.get(&session_id);
        if let Some(session) = session {
            let _ = session.client.do_send(message);
        }
    }

    fn send_user_message(&self, user: u64, message: Message) {
        let sessions = self.sessions_by_user.get(&user);
        if let Some(sessions) = sessions {
            for session in sessions {
                let session = self.sessions.get(&session);
                if let Some(session) = session {
                    let _ = session.client.do_send(message.clone());
                }
            }
        }
    }

    fn leave_room(
        &mut self,
        session_id: usize,
        room_id: Option<u32>,
    ) -> impl ActorFuture<Output = (), Actor = Self> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .expect("session not found");
        let rooms = &self.rooms;
        let rooms: Vec<_> = if let Some(room_id) = room_id {
            session.room_ids.retain(|id| *id != room_id);
            rooms.get(&room_id).cloned().into_iter().collect()
        } else {
            session
                .room_ids
                .drain(..)
                .filter_map(|id| rooms.get(&id))
                .cloned()
                .collect()
        };

        let fut = async move {
            for room in rooms {
                let _ = room.addr.send(game_room::Leave { session_id }).await;
            }
        };

        fut.into_actor(self)
    }

    fn join_room(
        &mut self,
        session_id: usize,
        room_id: u32,
    ) -> impl ActorFuture<Output = (), Actor = Self> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .expect("session not found");
        let user_id = session.user_id.expect("user_id not set in Join");
        let room_addr = self.rooms.get(&room_id).map(|r| r.addr.clone());
        let addr = session.game_client.clone();

        let prefetch = if let Some(room_addr) = room_addr {
            session.room_ids.push(room_id);
            fut::Either::Right(async move { Ok::<_, ()>(room_addr) }.into_actor(self))
        } else {
            fut::Either::Left(
                self.db
                    .send(db::GetGame(room_id as _))
                    .into_actor(self)
                    .then(move |res, act, ctx| {
                        let db_game = match res {
                            Ok(Ok(db_game)) => db_game,
                            _ => return fut::err(()),
                        };

                        let replay = match db_game.replay {
                            Some(r) => r,
                            _ => return fut::err(()),
                        };

                        let game = match game::Game::load(&replay) {
                            Some(r) => r,
                            _ => return fut::err(()),
                        };

                        let room = GameRoom {
                            room_id,
                            owner: db_game.owner.map(|x| x as _),
                            sessions: HashMap::new(),
                            users: HashSet::new(),
                            name: db_game.name.to_owned(),
                            last_action: Instant::now(),
                            game,
                            db: act.db.clone(),
                            server: ctx.address(),
                            kicked_players: HashSet::new(),
                        };

                        let addr = room.start();

                        act.rooms.insert(
                            room_id,
                            Room {
                                addr: addr.clone(),
                                name: db_game.name.to_owned(),
                            },
                        );

                        let session = act
                            .sessions
                            .get_mut(&session_id)
                            .expect("session not found");
                        session.room_ids.push(room_id);

                        fut::ok(addr)
                    }),
            )
        };

        prefetch.then(move |res, act, _| {
            if let Ok(room_addr) = res {
                room_addr.do_send(game_room::Join {
                    session_id,
                    user_id,
                    addr,
                });
            }
            async {}.into_actor(act)
        })
    }
}

impl Actor for GameServer {
    type Context = Context<Self>;

    fn stopping(&mut self, _ctx: &mut Self::Context) -> Running {
        println!("Server stopping!");
        Running::Stop
    }
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(
            id,
            Session {
                user_id: None,
                client: msg.addr,
                game_client: msg.game_addr,
                room_ids: Vec::new(),
            },
        );

        // TODO: the client DOES NOT  need to know every profile..
        for user_id in self.sessions_by_user.keys() {
            let profile = self.profiles.get(user_id).unwrap();
            self.send_message(id, Message::UpdateProfile(profile.clone()));
        }

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, ctx: &mut Context<Self>) {
        println!("Someone disconnected");

        self.leave_room(msg.id, None)
            .then(move |(), act, _| {
                // remove address
                if let Some(session) = act.sessions.remove(&msg.id) {
                    let empty = if let Some(sessions) = session
                        .user_id
                        .and_then(|uid| act.sessions_by_user.get_mut(&uid))
                    {
                        sessions.retain(|&s| s != msg.id);
                        sessions.is_empty()
                    } else {
                        false
                    };

                    if empty {
                        act.sessions_by_user
                            .remove(session.user_id.as_ref().unwrap());
                    }
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

/// Handler for `ListRooms` message.
impl Handler<ListRooms> for GameServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for (&key, room) in &self.rooms {
            rooms.push((key, room.name.clone()));
        }

        MessageResult(rooms)
    }
}

/// Join room, send disconnect message to old room
impl Handler<Join> for GameServer {
    type Result = ActorResponse<Self, Addr<GameRoom>, ()>;

    fn handle(&mut self, msg: Join, _ctx: &mut Context<Self>) -> Self::Result {
        let Join {
            id,
            room_id,
            leave_previous,
        } = msg;

        let session = match self.sessions.get(&id) {
            Some(x) => x,
            None => return ActorResponse::reply(Err(())),
        };

        match session.user_id {
            Some(x) => x,
            None => return ActorResponse::reply(Err(())),
        };

        let after_leave = if leave_previous {
            fut::Either::Left(self.leave_room(msg.id, None))
        } else {
            fut::Either::Right(async {}.into_actor(self))
        };

        let result = after_leave
            .then(move |(), act, _ctx| act.join_room(id, room_id))
            .then(move |(), act, _ctx| {
                fut::ready(match act.rooms.get(&room_id) {
                    Some(room) => Ok(room.addr.clone()),
                    None => Err(()),
                })
            });

        ActorResponse::r#async(result)
    }
}

impl Handler<LeaveRoom> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: LeaveRoom, ctx: &mut Context<Self>) -> Self::Result {
        let LeaveRoom { id, room_id } = msg;

        let session = match self.sessions.get(&id) {
            Some(x) => x,
            None => return,
        };

        match session.user_id {
            Some(x) => x,
            None => return,
        };

        self.leave_room(msg.id, room_id).wait(ctx)
    }
}

/// Create room, announce to users
impl Handler<CreateRoom> for GameServer {
    type Result = ActorResponse<Self, (u32, Option<Addr<GameRoom>>), message::Error>;

    fn handle(&mut self, msg: CreateRoom, _: &mut Context<Self>) -> Self::Result {
        use message::Error;
        let CreateRoom {
            id,
            room:
                message::StartGame {
                    name,
                    seats,
                    komis,
                    size,
                    mods,
                },
            leave_previous,
        } = msg;

        if name.len() > 50 {
            return ActorResponse::reply(Err(Error::other("Name too long")));
        }

        let session = if id == 0 {
            None
        } else {
            match self.sessions.get(&id) {
                Some(x) => Some(x),
                None => return ActorResponse::reply(Err(Error::other("No session"))),
            }
        };

        let user_id = if let Some(session) = session {
            match session.user_id {
                Some(x) => x,
                None => return ActorResponse::reply(Err(Error::other("Not identified"))),
            }
        } else {
            0
        };

        let komis = komis.as_slice().into();
        let seed = self.rng.next_u64();
        let game = match game::Game::standard(&seats, komis, size, mods, seed) {
            Some(g) => g,
            None => return ActorResponse::reply(Err(Error::other("Rules not accepted"))),
        };

        if user_id != 0 {
            let profile = self
                .profiles
                .get_mut(&user_id)
                .expect("User id exists without session");

            if let Some(time) = profile.last_game_time {
                // Only allow creating a game once every two minutes.
                let diff = Instant::now() - time;
                let target = Duration::from_secs(60 * 2);
                if diff < target {
                    return ActorResponse::reply(Err(Error::GameStartTimer(
                        (target - diff).as_secs(),
                    )));
                }
            }

            profile.last_game_time = Some(Instant::now());
        }

        let cloned_name = name.clone();

        let owner = if user_id != 0 { Some(user_id) } else { None };

        let after_leave = if leave_previous {
            fut::Either::Left(self.leave_room(msg.id, None))
        } else {
            fut::Either::Right(async {}.into_actor(self))
        };
        let result = after_leave
            .then(move |(), act, _| {
                act.db
                    .send(db::StoreGame {
                        id: None,
                        replay: None,
                        name: cloned_name,
                        owner,
                    })
                    .into_actor(act)
            })
            .then(move |res, act, ctx| {
                let room_id = match res {
                    Ok(Ok(g)) => g.id as _,
                    _ => {
                        return fut::Either::Left(
                            async { Err(Error::other("Internal error")) }.into_actor(act),
                        )
                    }
                };

                let room = GameRoom {
                    room_id,
                    owner,
                    sessions: HashMap::new(),
                    users: HashSet::new(),
                    name: name.clone(),
                    last_action: Instant::now(),
                    game,
                    db: act.db.clone(),
                    server: ctx.address(),
                    kicked_players: HashSet::new(),
                };

                let addr = room.start();

                act.rooms.insert(
                    room_id,
                    Room {
                        addr: addr.clone(),
                        name: name.clone(),
                    },
                );

                act.send_global_message(Message::AnnounceRoom(room_id, name));

                fut::Either::Right(if user_id == 0 {
                    fut::Either::Left(fut::ready(Ok((room_id, Some(addr)))))
                } else {
                    fut::Either::Right(
                        act.join_room(id, room_id)
                            .then(move |(), _, _| fut::ready(Ok((room_id, Some(addr))))),
                    )
                })
            });

        ActorResponse::r#async(result)
    }
}

impl Handler<IdentifyAs> for GameServer {
    type Result = ActorResponse<Self, Profile, message::Error>;

    fn handle(&mut self, msg: IdentifyAs, _ctx: &mut Self::Context) -> Self::Result {
        use message::Error;

        let IdentifyAs { id, token, nick } = msg;

        if let Some(nick) = &nick {
            let nick = nick.trim();
            if nick.len() >= 30 {
                return ActorResponse::r#async(fut::err(Error::other("Nickname too long")));
            }
            if nick.is_empty() {
                return ActorResponse::r#async(fut::err(Error::other("Nickname can't be empty")));
            }
        }

        let rng = &mut self.rng;

        let token = token
            .and_then(|t| Uuid::parse_str(&t).ok())
            .unwrap_or_else(|| Uuid::from_bytes(rng.gen()));

        let db = self.db.clone();
        let fut = db.send(db::IdentifyUser {
            auth_token: token.to_string(),
            nick: nick.clone(),
        });

        let fut = fut.into_actor(self).then(move |res, act, _| {
            let user = match res {
                Ok(Ok(u)) => u,
                _ => return fut::err(Error::other("No profile")),
            };

            let user_id = user.id as u64;

            let profile = act.profiles.entry(user_id).or_insert_with(move || Profile {
                user_id,
                token,
                nick: user.nick,
                last_game_time: None,
                is_admin: false,
            });

            if let Some(nick) = nick {
                let nick = nick.trim();
                // The nick has already been sanitized at this point.
                if nick.is_empty() {
                    profile.nick = None;
                } else {
                    profile.nick = Some(nick.to_owned());
                }
            }

            profile.is_admin = profile.token == act.admin_token;

            let profile = profile.clone();

            act.send_user_message(user_id, Message::Identify(profile.clone()));

            let sessions = act
                .sessions_by_user
                .entry(user_id)
                .or_insert_with(HashSet::new);
            sessions.insert(id);

            catch! {
                act.sessions.get_mut(&id)?.user_id = Some(user_id);
            };

            // Announce profile update to users
            // TODO: only send the profile to users in relevant rooms
            act.send_global_message(Message::UpdateProfile(profile.clone()));

            fut::ok(profile)
        });

        ActorResponse::r#async(fut)
    }
}

impl Handler<QueryProfile> for GameServer {
    type Result = ActorResponse<Self, Profile, ()>;

    fn handle(&mut self, msg: QueryProfile, _ctx: &mut Self::Context) -> Self::Result {
        let QueryProfile { user_id } = msg;

        // TODO: Cache the profile here.

        let fut = self.db.send(db::GetUser(user_id));

        let fut = fut.into_actor(self).then(move |res, act, _| {
            let user = match res {
                Ok(Ok(u)) => u,
                _ => return fut::err(()),
            };

            let token = Uuid::parse_str(&user.auth_token).unwrap_or_else(|_| Uuid::default());
            let is_admin = token == act.admin_token;

            let profile = Profile {
                user_id: user.id as u64,
                token,
                nick: user.nick,
                last_game_time: None,
                is_admin,
            };

            // TODO: only send the profile to users in relevant rooms
            // TODO: don't send this here but in the room actor
            act.send_global_message(Message::UpdateProfile(profile.clone()));

            fut::ok(profile)
        });

        ActorResponse::r#async(fut)
    }
}

impl Handler<AdminMessage> for GameServer {
    type Result = MessageResult<AdminMessage>;

    fn handle(&mut self, msg: AdminMessage, _: &mut Context<Self>) -> Self::Result {
        let AdminMessage { client_id, action } = msg;

        macro_rules! r {
            ($x:expr) => {
                match $x {
                    Some(x) => x,
                    None => return MessageResult(()),
                }
            };
        }

        let session = r!(self.sessions.get(&client_id));
        let user_id = r!(session.user_id);
        let profile = r!(self.profiles.get(&user_id));

        if !profile.is_admin {
            return MessageResult(());
        }

        match action {
            AdminAction::UnloadRoom(room_id) => {
                let room = r!(self.rooms.remove(&room_id));
                room.addr.do_send(game_room::Unload);
                self.send_global_message(Message::CloseRoom(room_id));
            }
        }

        MessageResult(())
    }
}

impl Handler<GetAdminView> for GameServer {
    type Result = ActorResponse<Self, game::GameView, ()>;

    fn handle(&mut self, msg: GetAdminView, _ctx: &mut Self::Context) -> Self::Result {
        let GetAdminView { room_id } = msg;

        let room = match self.rooms.get(&room_id) {
            Some(x) => x,
            None => return ActorResponse::reply(Err(())),
        };
        let fut = room.addr.send(game_room::GetAdminView);

        let fut = fut.into_actor(self).then(move |res, _act, _| {
            let view = match res {
                Ok(Ok(v)) => v,
                _ => return fut::err(()),
            };

            fut::ok(view)
        });

        ActorResponse::r#async(fut)
    }
}
