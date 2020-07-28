
use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};
use uuid::Uuid;
use std::collections::{HashMap, HashSet};

use crate::message;

macro_rules! catch {
    ($($code:tt)+) => {
        (|| Some({ $($code)+ }))()
    };
}

/// Server sends this when a new room is created
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    AnnounceRoom(u32),
    GameStatus {
        room_id: u32,
        members: Vec<u64>,
        moves: Vec<(u32, u32)>
    },
    Identify(Profile),
    UpdateProfile(Profile),
}

/// New chat session is created
#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
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
    type Result = Vec<u32>;
}

/// Join room, if room does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client id
    pub id: usize,
    /// Room name
    pub room_id: u32,
}

/// Create room, announce to clients
pub struct CreateRoom {
    /// Client id
    pub id: usize,
}

impl actix::Message for CreateRoom {
    type Result = Option<u32>;
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct GameAction {
    pub id: usize,
    pub room_id: u32,
    pub action: message::GameAction
}

#[derive(Message)]
#[rtype(Profile)]
pub struct IdentifyAs {
    pub id: usize,
    pub token: Option<String>,
    pub nick: Option<String>
}

#[derive(Clone)]
pub struct Profile {
    pub user_id: u64,
    pub token: Uuid,
    pub nick: Option<String>
}

pub struct Session {
    pub user_id: Option<u64>,
    pub client: Recipient<Message>
}

pub struct Room {
    members: HashSet<usize>,
    users: HashSet<u64>,
    moves: Vec<(u32, u32)>
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct GameServer {
    sessions: HashMap<usize, Session>,
    sessions_by_user: HashMap<u64, Vec<usize>>,
    profiles: HashMap<u64, Profile>,
    user_tokens: HashMap<Uuid, u64>,
    rooms: HashMap<u32, Room>,
    rng: ThreadRng,
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
        let room = self.rooms.get(&room)?;
        for user in &room.members {
            let session = self.sessions.get(&user);
            if let Some(session) = session {
                let _ = session.client.do_send(message.clone());
            }
        }
        Some(())
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

    fn leave_room(&mut self, session_id: usize, room_id: u32) {
        let mut user_removed = false;

        if let Some(session) = self.sessions.get(&session_id) {
            // remove session from all rooms
            if let Some(room) = self.rooms.get_mut(&room_id) {
                if room.members.remove(&session_id) {
                    if let Some(user_id) = session.user_id {
                        let sessions = &self.sessions;
                        if !room.members.iter()
                            .any(|s| sessions.get(s).unwrap().user_id == Some(user_id)) {
                            room.users.remove(&user_id);
                            user_removed = true;
                        }
                    }
                }
            }
        }

        if user_removed {
            if let Some(room) = self.rooms.get(&room_id) {
                let msg = Message::GameStatus {
                    room_id,
                    members: room.users.iter().copied().collect(),
                    moves: room.moves.clone()
                };
                self.send_room_message(room_id, msg);
            }
        }
    }
}

/// Make actor from `ChatServer`
impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
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
        self.sessions.insert(id, Session {
            user_id: None,
            client: msg.addr
        });

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        let mut rooms = Vec::new();

        // remove session from all rooms
        for (room_id, room) in &mut self.rooms {
            if room.members.contains(&msg.id) {
                rooms.push(*room_id);
            }
        }

        for room_id in rooms {
            self.leave_room(msg.id, room_id)
        }

        // remove address
        if let Some(session) = self.sessions.remove(&msg.id) {
            if let Some(sessions) = session.user_id
                .and_then(|uid| self.sessions_by_user.get_mut(&uid)) {
                sessions.retain(|&s| s != msg.id);
            }
        }
    }
}

/// Handler for `ListRooms` message.
impl Handler<ListRooms> for GameServer {
    type Result = MessageResult<ListRooms>;

    fn handle(&mut self, _: ListRooms, _: &mut Context<Self>) -> Self::Result {
        let mut rooms = Vec::new();

        for key in self.rooms.keys() {
            rooms.push(key.to_owned())
        }

        MessageResult(rooms)
    }
}

/// Join room, send disconnect message to old room
impl Handler<Join> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, room_id } = msg;

        let user_id = match catch!(self.sessions.get(&id)?.user_id?) {
            Some(x) => x,
            None => return
        };

        let mut rooms = Vec::new();

        // remove session from all rooms
        for (room_id, room) in &mut self.rooms {
            if room.members.contains(&id) {
                rooms.push(*room_id);
            }
        }
        for room_id in rooms {
            self.leave_room(msg.id, room_id)
        }

        catch!{
            let room = self.rooms.get_mut(&room_id)?;
            room.members.insert(id);
            room.users.insert(user_id);
            let msg = Message::GameStatus {
                room_id,
                members: room.users.iter().copied().collect(),
                moves: room.moves.clone()
            };
            self.send_room_message(room_id, msg);
        };
    }
}

/// Create room, announce to users
impl Handler<CreateRoom> for GameServer {
    type Result = MessageResult<CreateRoom>;

    fn handle(&mut self, msg: CreateRoom, _: &mut Context<Self>) -> Self::Result {
        let CreateRoom { id } = msg;

        let user_id = match catch!(self.sessions.get(&id)?.user_id?) {
            Some(x) => x,
            None => return MessageResult(None)
        };

        let mut rooms = Vec::new();

        // remove session from all rooms
        for (room_id, room) in &mut self.rooms {
            if room.members.contains(&id) {
                rooms.push(*room_id);
            }
        }
        for room_id in rooms {
            self.leave_room(id, room_id)
        }

        let room_id = self.rng.gen();

        let mut room = Room {
            members: HashSet::new(),
            users: HashSet::new(),
            moves: Vec::new()
        };
        room.members.insert(id);
        room.users.insert(user_id);

        self.send_message(id, Message::GameStatus {
            room_id,
            members: room.users.iter().copied().collect(),
            moves: room.moves.clone()
        });

        self.rooms
            .insert(room_id, room);

        self.send_global_message(Message::AnnounceRoom(room_id));

        MessageResult(Some(room_id))
    }
}

impl Handler<GameAction> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: GameAction, _: &mut Context<Self>) {
        let GameAction { id, room_id, action } = msg;

        match self.rooms.get_mut(&room_id) {
            Some(room) => {
                match action {
                    message::GameAction::Place(x, y) => room.moves.push((x, y))
                }
            },
            None => {}
        };

        match self.rooms.get(&room_id) {
            Some(room) => {
                self.send_room_message(room_id, Message::GameStatus {
                    room_id,
                    members: room.users.iter().copied().collect(),
                    moves: room.moves.clone()
                });
            },
            None => {}
        };
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

        let profile = self.profiles
            .entry(user_id)
            .or_insert_with(|| Profile {
                user_id,
                token,
                nick: None
            });

        if let Some(nick) = nick {
            profile.nick = Some(nick);
        }

        let profile = profile.clone();

        self.send_user_message(user_id, Message::Identify(profile.clone()));

        let sessions = self.sessions_by_user.entry(user_id).or_insert_with(|| Vec::new());
        sessions.push(id);

        catch!{
            self.sessions.get_mut(&id)?.user_id = Some(user_id);
        };

        MessageResult(profile)
    }
}

