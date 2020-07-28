
#[derive(Clone, PartialEq)]
pub struct GameView {
    pub members: Vec<u64>,
    pub moves: Vec<(u32, u32)>
}

#[derive(Clone, PartialEq)]
pub struct Profile {
    pub user_id: u64,
    pub nick: Option<String>
}

