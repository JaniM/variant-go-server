use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum ClientMessage {
    GetGameList,
    StartGame
}

#[derive(Serialize, Deserialize, Debug)]
pub enum ServerMessage {
    GameList {
        games: Vec<u32>
    },
    MsgError(String)
}
