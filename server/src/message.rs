use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    Place(u32, u32),
    TakeSeat(u32),
    LeaveSeat(u32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    Identify {
        token: Option<String>,
        nick: Option<String>
    },
    GetGameList,
    JoinGame(u32),
    GameAction(GameAction),
    StartGame
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Profile {
    pub user_id: u64,
    pub nick: Option<String>
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    Identify {
        token: String,
        nick: Option<String>,
        user_id: u64
    },
    GameList {
        games: Vec<u32>
    },
    GameStatus {
        room_id: u32,
        members: Vec<u64>,
        seats: Vec<(Option<u64>, u8)>,
        turn: u32,
        // 19x19 vec, 0 = empty, 1 = black, 2 = white
        board: Vec<u8>
    },
    Profile(Profile),
    MsgError(String)
}
