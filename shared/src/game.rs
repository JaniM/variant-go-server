use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use bitmaps::Bitmap;
use tinyvec::TinyVec;

use crate::assume::AssumeFrom;
use crate::states::{FreePlacement, PlayState, ScoringState};

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Color(pub u8);

impl Color {
    pub const fn empty() -> Color {
        Color(0)
    }

    pub const fn is_empty(self) -> bool {
        self.0 == 0
    }

    pub const fn as_usize(self) -> usize {
        self.0 as usize
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::empty()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Seat {
    pub player: Option<u64>,
    pub team: Color,
}

impl Seat {
    fn new(color: Color) -> Seat {
        Seat {
            player: None,
            team: color,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ActionKind {
    Place(u32, u32),
    Pass,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReplayActionKind {
    Play(ActionKind),
    TakeSeat(u32),
    LeaveSeat(u32),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameAction {
    pub user_id: u64,
    pub action: ReplayActionKind,
}

impl GameAction {
    fn new(user_id: u64, action: ReplayActionKind) -> Self {
        GameAction { user_id, action }
    }

    fn play(user_id: u64, action: ActionKind) -> Self {
        GameAction::new(user_id, ReplayActionKind::Play(action))
    }
}

#[derive(Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub struct Board<T = Color> {
    pub width: u32,
    pub height: u32,
    pub points: Vec<T>,
}

pub type Point = (u32, u32);

impl<T: Copy + Default> Board<T> {
    pub fn empty(width: u32, height: u32) -> Self {
        Board {
            width,
            height,
            points: vec![T::default(); (width * height) as usize],
        }
    }

    pub fn point_within(&self, (x, y): Point) -> bool {
        (0..self.width).contains(&x) && (0..self.height).contains(&y)
    }

    pub fn get_point(&self, (x, y): Point) -> T {
        self.points[(y * self.width + x) as usize]
    }

    pub fn point_mut(&mut self, (x, y): Point) -> &mut T {
        &mut self.points[(y * self.width + x) as usize]
    }

    pub fn idx_to_coord(&self, idx: usize) -> Option<Point> {
        if idx < self.points.len() {
            Some((idx as u32 % self.width, idx as u32 / self.width))
        } else {
            None
        }
    }

    pub fn surrounding_points(&self, p: Point) -> impl Iterator<Item = Point> {
        let x = p.0 as i32;
        let y = p.1 as i32;
        let width = self.width;
        let height = self.height;
        [(-1, 0), (1, 0), (0, -1), (0, 1)]
            .iter()
            .filter_map(move |&(dx, dy)| {
                if (x + dx) >= 0 && x + dx < width as i32 && (y + dy) >= 0 && y + dy < height as i32
                {
                    Some(((x + dx) as u32, (y + dy) as u32))
                } else {
                    None
                }
            })
    }
}

impl<T: Hash> Board<T> {
    pub fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(&self, &mut hasher);
        hasher.finish()
    }
}

pub type GroupVec<T> = TinyVec<[T; 8]>;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub points: GroupVec<Point>,
    pub liberties: i32,
    pub team: Color,
    pub alive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameState {
    FreePlacement(FreePlacement),
    Play(PlayState),
    Scoring(ScoringState),
    Done(ScoringState),
}

impl GameState {
    pub fn free_placement(
        seat_count: usize,
        team_count: usize,
        board: Board,
        teams_share_stones: bool,
    ) -> Self {
        GameState::FreePlacement(FreePlacement::new(
            seat_count,
            team_count,
            board,
            teams_share_stones,
        ))
    }

    pub fn play(seat_count: usize) -> Self {
        GameState::Play(PlayState::new(seat_count))
    }

    pub fn scoring(board: &Board, seat_count: usize, scores: &[i32]) -> Self {
        GameState::Scoring(ScoringState::new(board, seat_count, scores))
    }
}

assume!(GameState);
assume!(GameState, Play(x) => x, PlayState);
assume!(GameState, Scoring(x) => x, ScoringState);
assume!(GameState, FreePlacement(x) => x, FreePlacement);

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct ZenGo {
    pub color_count: u8,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HiddenMoveGo {
    pub placement_count: u32,
    pub teams_share_stones: bool,
}

/// Visibility modes describe how the game state should be displayed, without
/// affecting the actual gameplay in any way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum VisibilityMode {
    /// Display all stones as the same color for both players.
    OneColor,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GameModifier {
    /// Pixel go is a game mode where you place 2x2 blobs instead of a single stone.
    /// Overlapping existing stones are ignored.
    /// The blob must fit on the board.
    pub pixel: bool,

    /// "Ponnuki is 30 points". Whenever a player captures a single stone, forming a ponnuki
    /// they get (or lose) points.
    #[serde(default)]
    pub ponnuki_is_points: Option<i32>,

    #[serde(default)]
    pub zen_go: Option<ZenGo>,

    #[serde(default)]
    pub hidden_move: Option<HiddenMoveGo>,

    #[serde(default)]
    pub visibility_mode: Option<VisibilityMode>,

    /// Prevents looking at history during the game. Especially handy for one color go.
    #[serde(default)]
    pub no_history: bool,
}

pub type Visibility = Bitmap<typenum::U16>;
pub type VisibilityBoard = Board<Bitmap<typenum::U16>>;

#[derive(Debug, Clone, PartialEq)]
pub struct BoardHistory {
    pub hash: u64,
    pub board: Board,
    pub board_visibility: Option<VisibilityBoard>,
    pub state: GameState,
    pub points: GroupVec<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SharedState {
    pub seats: GroupVec<Seat>,
    pub points: GroupVec<i32>,
    pub turn: usize,
    pub pass_count: usize,
    pub board: Board,
    pub board_visibility: Option<VisibilityBoard>,
    pub board_history: Vec<BoardHistory>,
    /// Optimization for superko
    pub capture_count: usize,
    pub komis: GroupVec<i32>,
    pub mods: GameModifier,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub state: GameState,
    pub state_stack: Vec<GameState>,
    pub shared: SharedState,
    pub actions: Vec<GameAction>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TakeSeatError {
    DoesNotExist,
    NotOpen,
    CanOnlyHoldOne,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum MakeActionError {
    NotPlayer,
    NotTurn,
    OutOfBounds,
    PointOccupied,
    Suicide,
    Ko,
    GameDone,
}

pub enum ActionChange {
    None,
    SwapState(GameState),
    PushState(GameState),
    PopState,
}

pub type MakeActionResult<T = ActionChange> = Result<T, MakeActionError>;

#[derive(Debug, Clone, PartialEq)]
pub struct GameView {
    // TODO: we need a separate state view since we have hidden information
    // currently players can cheat :F
    pub state: GameState,
    pub seats: GroupVec<Seat>,
    pub turn: u32,
    pub board: Vec<Color>,
    pub board_visibility: Option<Vec<Visibility>>,
    pub hidden_stones_left: u32,
    pub size: (u8, u8),
    pub mods: GameModifier,
    pub points: GroupVec<i32>,
    pub move_number: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct GameHistory {
    pub board: Vec<u8>,
    pub board_visibility: Option<Vec<u16>>,
    pub last_stone: Option<GroupVec<(u32, u32)>>,
    pub move_number: u32,
}

#[derive(Serialize, Deserialize)]
struct GameReplay {
    actions: Vec<GameAction>,
    mods: GameModifier,
    komis: GroupVec<i32>,
    seats: GroupVec<u8>,
    size: (u8, u8),
}

impl Game {
    pub fn standard(
        seats: &[u8],
        komis: GroupVec<i32>,
        size: (u8, u8),
        mods: GameModifier,
    ) -> Option<Game> {
        if !seats.iter().all(|&t| t > 0 && t <= 3) {
            return None;
        }

        if !(1..=7).contains(&seats.len()) || !(1..=3).contains(&komis.len()) {
            return None;
        }

        // Don't allow huge boards
        if size.0 > 19 || size.1 > 19 {
            return None;
        }

        let board = Board::empty(size.0 as _, size.1 as _);
        let state = if let Some(rules) = &mods.hidden_move {
            GameState::free_placement(
                seats.len(),
                komis.len(),
                board.clone(),
                rules.teams_share_stones,
            )
        } else {
            GameState::play(seats.len())
        };

        Some(Game {
            state,
            state_stack: Vec::new(),
            shared: SharedState {
                seats: seats.iter().map(|&t| Seat::new(Color(t))).collect(),
                points: komis.clone(),
                turn: 0,
                pass_count: 0,
                board: board.clone(),
                board_visibility: None,
                board_history: vec![BoardHistory {
                    hash: board.hash(),
                    board,
                    board_visibility: None,
                    state: GameState::play(seats.len()),
                    points: komis.clone(),
                }],
                capture_count: 0,
                komis,
                mods,
            },
            actions: vec![],
        })
    }

    /// Loads a game from a replay dump. Can fail at any point due to changed rules...
    /// Such is life.
    pub fn load(dump: &[u8]) -> Option<Game> {
        let replay: GameReplay = serde_cbor::from_slice(dump).ok()?;
        let mut game = Game::standard(&replay.seats, replay.komis, replay.size, replay.mods)?;

        for action in replay.actions {
            use ReplayActionKind::*;
            match action.action {
                TakeSeat(seat_id) => {
                    game.take_seat(action.user_id, seat_id as _).ok()?;
                }
                LeaveSeat(seat_id) => {
                    game.leave_seat(action.user_id, seat_id as _).ok()?;
                }
                Play(play) => {
                    game.make_action(action.user_id, play).ok()?;
                }
            }
        }

        Some(game)
    }

    /// Dumps the game to a (hopefully somewhat) stable replay format.
    pub fn dump(&self) -> Vec<u8> {
        let shared = &self.shared;
        let replay = GameReplay {
            actions: self.actions.clone(),
            komis: shared.komis.clone(),
            size: (shared.board.width as _, shared.board.height as _),
            seats: shared.seats.iter().map(|x| x.team.0).collect(),
            mods: shared.mods.clone(),
        };

        let mut vec = Vec::new();
        replay
            .serialize(&mut serde_cbor::Serializer::new(&mut vec).packed_format())
            .expect("Game dump failed");
        vec
    }

    pub fn take_seat(&mut self, player_id: u64, seat_id: usize) -> Result<(), TakeSeatError> {
        let shared = &mut self.shared;

        if shared.mods.hidden_move.is_some() {
            let held = shared.seats.iter().any(|x| x.player == Some(player_id));
            if held {
                return Err(TakeSeatError::CanOnlyHoldOne);
            }
        }

        let seat = shared
            .seats
            .get_mut(seat_id)
            .ok_or(TakeSeatError::DoesNotExist)?;
        if seat.player.is_some() {
            return Err(TakeSeatError::NotOpen);
        }
        seat.player = Some(player_id);
        self.actions.push(GameAction::new(
            player_id,
            ReplayActionKind::TakeSeat(seat_id as _),
        ));
        Ok(())
    }

    pub fn leave_seat(&mut self, player_id: u64, seat_id: usize) -> Result<(), TakeSeatError> {
        let shared = &mut self.shared;
        let seat = shared
            .seats
            .get_mut(seat_id)
            .ok_or(TakeSeatError::DoesNotExist)?;
        if seat.player != Some(player_id) {
            return Err(TakeSeatError::NotOpen);
        }
        seat.player = None;
        self.actions.push(GameAction::new(
            player_id,
            ReplayActionKind::LeaveSeat(seat_id as _),
        ));
        Ok(())
    }

    pub fn make_action(
        &mut self,
        player_id: u64,
        action: ActionKind,
    ) -> Result<(), MakeActionError> {
        if !self
            .shared
            .seats
            .iter()
            .any(|s| s.player == Some(player_id))
        {
            return Err(MakeActionError::NotPlayer);
        }

        let res = match &mut self.state {
            GameState::FreePlacement(state) => {
                state.make_action(&mut self.shared, player_id, action.clone())
            }
            GameState::Play(state) => {
                state.make_action(&mut self.shared, player_id, action.clone())
            }
            GameState::Scoring(state) => {
                state.make_action(&mut self.shared, player_id, action.clone())
            }
            GameState::Done(_) => Err(MakeActionError::GameDone),
        };

        match res {
            Ok(change) => {
                match change {
                    ActionChange::SwapState(new_state) => {
                        self.state = new_state;
                    }
                    ActionChange::PushState(new_state) => {
                        let old_state = std::mem::replace(&mut self.state, new_state);
                        self.state_stack.push(old_state);
                    }
                    ActionChange::PopState => {
                        self.state = self.state_stack.pop().expect("Empty state stack popped");
                    }
                    ActionChange::None => {}
                }

                self.actions.push(GameAction::play(player_id, action));

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn get_board_view(
        &self,
        player_id: u64,
        state: &GameState,
        board: &Board,
        board_visibility: &Option<VisibilityBoard>,
        game_done: bool,
    ) -> (Vec<Color>, Option<Vec<Visibility>>, u32) {
        let shared = &self.shared;

        let (board, board_visibility, hidden_stones_left) = match state {
            GameState::FreePlacement(state) => {
                if let Some((seat_idx, active_seat)) = shared
                    .seats
                    .iter()
                    .enumerate()
                    .find(|(_, x)| x.player == Some(player_id))
                {
                    let team = active_seat.team;

                    let board = if state.teams_share_stones {
                        &state.boards[team.0 as usize - 1]
                    } else {
                        &state.boards[seat_idx]
                    };
                    (board.points.clone(), None, 0)
                } else {
                    (shared.board.points.clone(), None, 0)
                }
            }
            GameState::Play(_) => {
                let mut board = board.points.clone();
                let board_visibility = board_visibility.clone();

                let one_color = matches!(
                    self.shared.mods.visibility_mode,
                    Some(VisibilityMode::OneColor)
                );

                // Set color to white.
                // TODO: Change this to black once the client supports selecting the color
                const ONE_COLOR_TEAM: Color = Color(2);

                // If the game is done, everything is visible.
                if game_done {
                    return (board, board_visibility.map(|x| x.points), 0);
                }

                if one_color {
                    for p in &mut board {
                        if !p.is_empty() {
                            *p = ONE_COLOR_TEAM;
                        }
                    }
                };

                if let Some(active_seat) = shared.seats.iter().find(|x| x.player == Some(player_id))
                {
                    let team = if !one_color {
                        active_seat.team
                    } else {
                        ONE_COLOR_TEAM
                    };

                    if let Some(mut visibility) = board_visibility {
                        let mut hidden_stones_left = 0;
                        for (board, visibility) in board.iter_mut().zip(&mut visibility.points) {
                            if visibility.get(team.as_usize()) {
                                *board = team;
                                if visibility.len() > 1 {
                                    hidden_stones_left += 1;
                                }
                                *visibility = Bitmap::new();
                                visibility.set(team.as_usize(), true);
                            } else if !visibility.is_empty() {
                                hidden_stones_left += 1;
                                *board = Color::empty();
                                *visibility = Bitmap::new();
                            }
                        }
                        (board, Some(visibility.points), hidden_stones_left)
                    } else {
                        (board, None, 0)
                    }
                } else {
                    if let Some(visibility) = &board_visibility {
                        for (a, b) in board.iter_mut().zip(&visibility.points) {
                            if !b.is_empty() {
                                *a = Color::empty();
                            }
                        }
                    }
                    (board, None, 0)
                }
            }
            GameState::Scoring(_) | GameState::Done(_) => (board.points.clone(), None, 0),
        };

        (board, board_visibility, hidden_stones_left)
    }

    pub fn get_view(&self, player_id: u64) -> GameView {
        let shared = &self.shared;
        let game_done = matches!(self.state, GameState::Done(_));
        let (board, board_visibility, hidden_stones_left) = self.get_board_view(
            player_id,
            &self.state,
            &shared.board,
            &shared.board_visibility,
            game_done,
        );
        GameView {
            state: self.state.clone(),
            seats: shared.seats.clone(),
            turn: shared.turn as _,
            board,
            board_visibility,
            hidden_stones_left,
            size: (shared.board.width as u8, shared.board.height as u8),
            mods: shared.mods.clone(),
            points: shared.points.clone(),
            move_number: shared.board_history.len() as u32 - 1,
        }
    }

    pub fn get_view_at(&self, player_id: u64, turn: u32) -> Option<GameHistory> {
        let shared = &self.shared;
        let BoardHistory {
            board,
            state,
            board_visibility,
            ..
        } = &shared.board_history.get(turn as usize)?;

        let game_done = matches!(self.state, GameState::Done(_));

        if !game_done && self.shared.mods.no_history {
            return None;
        }

        let (board, board_visibility, _hidden_stones_left) =
            self.get_board_view(player_id, state, board, board_visibility, game_done);

        Some(GameHistory {
            board: board.iter().map(|x| x.0).collect(),
            board_visibility: board_visibility.map(|b| b.iter().map(|x| x.into_value()).collect()),
            last_stone: state.assume::<PlayState>().last_stone.clone(),
            move_number: turn,
        })
    }
}

pub fn find_groups(board: &Board) -> Vec<Group> {
    let mut legal_points = board
        .points
        .iter()
        .enumerate()
        .filter_map(|(idx, c)| {
            if !c.is_empty() {
                board.idx_to_coord(idx)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    let mut seen = HashSet::new();
    let mut stack = VecDeque::new();
    let mut groups = Vec::new();

    while let Some(point) = legal_points.pop() {
        let mut group = Group::default();
        group.alive = true;
        group.team = board.get_point(point);
        if group.team.is_empty() {
            unreachable!("scanned an empty point");
        }

        stack.push_back(point);

        while let Some(point) = stack.pop_front() {
            group.points.push(point);
            for point in board.surrounding_points(point) {
                if !seen.insert(point) {
                    continue;
                }

                match board.get_point(point) {
                    x if x == group.team => {
                        stack.push_back(point);
                        legal_points.retain(|x| *x != point);
                    }
                    Color(0) => group.liberties += 1,
                    _ => {}
                }
            }
        }

        seen.clear();
        groups.push(group);
    }

    groups
}
