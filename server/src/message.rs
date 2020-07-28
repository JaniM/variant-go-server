use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    Place(u32, u32)
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
        moves: Vec<(u32, u32)>
    },
    Profile(Profile),
    MsgError(String)
}
