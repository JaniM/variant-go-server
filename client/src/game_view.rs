use crate::game::{clock::GameClock, GameHistory, GameModifier, GameStateView};

#[derive(Clone, PartialEq, Debug)]
pub struct GameView {
    pub room_id: u32,
    pub owner: u64,
    pub members: Vec<u64>,
    // id, color, resigned
    // FIXME: This is horrible and makes the code hard to read
    pub seats: Vec<(Option<u64>, u8, bool)>,
    pub turn: u32,
    // 19x19 vec, 0 = empty, 1 = black, 2 = white
    pub board: Vec<u8>,
    pub board_visibility: Option<Vec<u16>>,
    pub hidden_stones_left: u32,
    pub size: (u8, u8),
    pub state: GameStateView,
    pub mods: GameModifier,
    pub points: Vec<i32>,
    pub move_number: u32,
    pub history: Option<GameHistory>,
    pub clock: Option<GameClock>,
}

#[derive(Clone, PartialEq)]
pub struct Profile {
    pub user_id: u64,
    pub nick: Option<String>,
}

impl Profile {
    pub fn nick_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.nick.as_deref().unwrap_or(default)
    }
}
