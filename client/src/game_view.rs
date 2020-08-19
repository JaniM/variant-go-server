use crate::game::{GameModifier, GameState};

#[derive(Clone, PartialEq)]
pub struct GameView {
    pub room_id: u32,
    pub members: Vec<u64>,
    pub seats: Vec<(Option<u64>, u8)>,
    pub turn: u32,
    // 19x19 vec, 0 = empty, 1 = black, 2 = white
    pub board: Vec<u8>,
    pub size: (u8, u8),
    pub state: GameState,
    pub mods: GameModifier,
    pub points: Vec<i32>,
}

#[derive(Clone, PartialEq)]
pub struct Profile {
    pub user_id: u64,
    pub nick: Option<String>,
}

impl Profile {
    pub fn nick_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.nick.as_ref().map(|x| &**x).unwrap_or(default)
    }
}
