
use actix::prelude::*;
use rand::{self, rngs::ThreadRng, Rng};
use std::collections::{HashMap, HashSet};

/// Server sends this when a new room is created
#[derive(Message, Clone)]
#[rtype(result = "()")]
pub enum Message {
    AnnounceRoom(usize)
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
    type Result = Vec<usize>;
}

/// Join room, if room does not exists create new one.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    /// Client id
    pub id: usize,
    /// Room name
    pub room_id: usize,
}

/// Create room, announce to clients
#[derive(Message)]
#[rtype(usize)]
pub struct CreateRoom {
    /// Client id
    pub id: usize,
}

pub struct Room {
    members: HashSet<usize>
}

/// `ChatServer` manages chat rooms and responsible for coordinating chat
/// session. implementation is super primitive
pub struct ChatServer {
    sessions: HashMap<usize, Recipient<Message>>,
    rooms: HashMap<usize, Room>,
    rng: ThreadRng,
}

impl Default for ChatServer {
    fn default() -> ChatServer {
        let mut rooms = HashMap::new();

        ChatServer {
            sessions: HashMap::new(),
            rooms,
            rng: rand::thread_rng(),
        }
    }
}

impl ChatServer {
    /// Send message to all users
    fn send_global_message(&self, message: Message) {
        for addr in self.sessions.values() {
            let _ = addr.do_send(message.clone());
        }
    }
}

/// Make actor from `ChatServer`
impl Actor for ChatServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for ChatServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        let mut rooms = Vec::new();

        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all rooms
            for (room_id, room) in &mut self.rooms {
                if room.members.remove(&msg.id) {
                    rooms.push(room_id);
                }
            }
        }
    }
}

/// Handler for `ListRooms` message.
impl Handler<ListRooms> for ChatServer {
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
impl Handler<Join> for ChatServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, room_id } = msg;
        let mut rooms = Vec::new();

        // remove session from all rooms
        for (room_id, room) in &mut self.rooms {
            if room.members.remove(&id) {
                rooms.push(room_id);
            }
        }

        match self.rooms.get_mut(&room_id) {
            Some(room) => { room.members.insert(id); },
            None => {}
        };
    }
}

/// Create room, announce to users
impl Handler<CreateRoom> for ChatServer {
    type Result = MessageResult<CreateRoom>;

    fn handle(&mut self, msg: CreateRoom, _: &mut Context<Self>) -> Self::Result {
        let CreateRoom { id } = msg;
        let mut rooms = Vec::new();

        // remove session from all rooms
        for (room_id, room) in &mut self.rooms {
            if room.members.remove(&id) {
                rooms.push(room_id);
            }
        }

        let room_id = self.rng.gen::<usize>();

        let mut room = Room { members: HashSet::new() };
        room.members.insert(id);

        self.rooms
            .insert(room_id, room);

        self.send_global_message(Message::AnnounceRoom(room_id));

        MessageResult(room_id)
    }
}
