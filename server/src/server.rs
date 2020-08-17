use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};
use uuid::Uuid;

use crate::game;
use crate::game_room::{self, GameRoom};
use crate::message;

macro_rules! catch {
    ($($code:tt)+) => {
        (|| Some({ $($code)+ }))()
    };
}

// TODO: separate game rooms to their own actors to deal with load

#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    // TODO: Use a proper struct, not magic tuples
    AnnounceRoom(u32, String),
    CloseRoom(u32),
    Identify(Profile),
    UpdateProfile(Profile),
}

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

/// List of available rooms
pub struct ListRooms;

impl actix::Message for ListRooms {
    // TODO: Use a proper struct, not magic tuples
    type Result = Vec<(u32, String)>;
}

/// Join room, if room does not exists create new one.
pub struct Join {
    /// Client id
    pub id: usize,
    pub room_id: u32,
}

impl actix::Message for Join {
    type Result = Result<Addr<GameRoom>, ()>;
}

/// Create room, announce to clients
pub struct CreateRoom {
    /// Client id
    pub id: usize,
    /// Room name
    pub name: String,
    pub seats: Vec<u8>,
    pub komis: Vec<i32>,
    pub size: (u8, u8),
}

impl actix::Message for CreateRoom {
    type Result = Result<(u32, Addr<GameRoom>), ()>;
}

#[derive(Message)]
#[rtype(Profile)]
pub struct IdentifyAs {
    pub id: usize,
    pub token: Option<String>,
    pub nick: Option<String>,
}

#[derive(Clone)]
pub struct Profile {
    pub user_id: u64,
    pub token: Uuid,
    pub nick: Option<String>,
}

pub struct Session {
    pub user_id: Option<u64>,
    pub client: Recipient<Message>,
    pub game_client: Recipient<game_room::Message>,
    pub room_id: Option<u32>,
}

pub struct Room {
    pub addr: Addr<GameRoom>,
    pub name: String,
}

/// `GameServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct GameServer {
    sessions: HashMap<usize, Session>,
    sessions_by_user: HashMap<u64, HashSet<usize>>,
    profiles: HashMap<u64, Profile>,
    user_tokens: HashMap<Uuid, u64>,
    rooms: HashMap<u32, Room>,
    rng: ThreadRng,
    game_counter: u32,
}

impl Default for GameServer {
    fn default() -> GameServer {
        let mut rooms = HashMap::new();

        GameServer {
            sessions: HashMap::new(),
            sessions_by_user: HashMap::new(),
            profiles: HashMap::new(),
            user_tokens: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
            game_counter: 0,
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

    /// Send message to all users in a room
    fn send_room_message(&self, room: u32, message: Message) -> Option<()> {
        todo!()
    }

    fn send_message(&self, session_id: usize, message: Message) {
        let session = self.sessions.get(&session_id);
        if let Some(session) = session {
            let _ = session.client.do_send(message.clone());
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

    fn leave_room(&mut self, session_id: usize) -> impl ActorFuture<Output = (), Actor = Self> {
        let session = self
            .sessions
            .get_mut(&session_id)
            .expect("session not found");
        let rooms = &self.rooms;
        let room_addr = catch!(rooms.get(&session.room_id?)?.addr.clone());

        if room_addr.is_some() {
            session.room_id = None;
        }

        let fut = async move {
            if let Some(room_addr) = room_addr {
                room_addr.send(game_room::Leave { session_id }).await;
            } else {
                ()
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

        if room_addr.is_some() {
            session.room_id = Some(room_id);
        }

        let fut = async move {
            if let Some(room_addr) = room_addr {
                room_addr
                    .send(game_room::Join {
                        session_id,
                        user_id,
                        addr,
                    })
                    .await;
            } else {
                ()
            }
        };

        fut.into_actor(self)
    }
}

impl Actor for GameServer {
    type Context = Context<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {}
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
                room_id: None,
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

        self.leave_room(msg.id)
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
    // Can this possibly be right?
    type Result = ActorResponse<Self, Addr<GameRoom>, ()>;

    fn handle(&mut self, msg: Join, ctx: &mut Context<Self>) -> Self::Result {
        let Join { id, room_id } = msg;

        let result = self
            .leave_room(msg.id)
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

/// Create room, announce to users
impl Handler<CreateRoom> for GameServer {
    type Result = ActorResponse<Self, (u32, Addr<GameRoom>), ()>;

    fn handle(&mut self, msg: CreateRoom, _: &mut Context<Self>) -> Self::Result {
        let CreateRoom {
            id,
            name,
            seats,
            komis,
            size,
        } = msg;

        // TODO: prevent spamming rooms (allow only one?)

        if name.len() > 50 {
            return ActorResponse::reply(Err(()));
        }

        let _user_id = match catch!(self.sessions.get(&id)?.user_id?) {
            Some(x) => x,
            None => return ActorResponse::reply(Err(())),
        };

        let game = match game::Game::standard(&seats, komis, size) {
            Some(g) => g,
            None => return ActorResponse::reply(Err(())),
        };

        let result = self.leave_room(id).then(move |(), act, _| {
            // TODO: room ids are currently sequential as a hack for ordering..
            let room_id = act.game_counter;
            act.game_counter += 1;

            let room = GameRoom {
                room_id,
                sessions: HashMap::new(),
                users: HashSet::new(),
                name: name.clone(),
                last_action: Instant::now(),
                game,
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

            act.join_room(id, room_id)
                .then(move |(), _, _| fut::ready(Ok((room_id, addr))))
        });

        ActorResponse::r#async(result)
    }
}

impl Handler<IdentifyAs> for GameServer {
    type Result = MessageResult<IdentifyAs>;

    fn handle(&mut self, msg: IdentifyAs, _: &mut Self::Context) -> Self::Result {
        let IdentifyAs { id, token, nick } = msg;

        let rng = &mut self.rng;

        let token = token
            .and_then(|t| Uuid::parse_str(&t).ok())
            .unwrap_or_else(|| Uuid::from_bytes(rng.gen()));
        let user_id = *self.user_tokens.entry(token).or_insert_with(|| rng.gen());

        let profile = self.profiles.entry(user_id).or_insert_with(|| Profile {
            user_id,
            token,
            nick: None,
        });

        if let Some(nick) = nick {
            if nick.len() < 30 {
                profile.nick = Some(nick);
            }
        }

        let profile = profile.clone();

        self.send_user_message(user_id, Message::Identify(profile.clone()));

        let sessions = self
            .sessions_by_user
            .entry(user_id)
            .or_insert_with(|| HashSet::new());
        sessions.insert(id);

        catch! {
            self.sessions.get_mut(&id)?.user_id = Some(user_id);
        };

        // Announce profile update to users
        // TODO: only send the profile to users in relevant rooms
        self.send_global_message(Message::UpdateProfile(profile.clone()));

        MessageResult(profile)
    }
}
