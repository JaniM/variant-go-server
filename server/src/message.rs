use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum GameAction {
    Place(u32, u32)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ClientMessage {
    GetGameList,
    JoinGame(u32),
    GameAction(GameAction),
    StartGame
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ServerMessage {
    GameList {
        games: Vec<u32>
    },
    GameStatus {
        room_id: u32,
        moves: Vec<(u32, u32)>
    },
    MsgError(String)
}
