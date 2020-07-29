
#[derive(Clone, PartialEq)]
pub struct GameView {
    pub members: Vec<u64>,
    pub seats: Vec<(Option<u64>, u8)>,
    pub turn: u32,
    // 19x19 vec, 0 = empty, 1 = black, 2 = white
    pub board: Vec<u8>,
}

#[derive(Clone, PartialEq)]
pub struct Profile {
    pub user_id: u64,
    pub nick: Option<String>
}

