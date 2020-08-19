use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Debug, Copy, Clone, PartialEq, Hash, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Color(pub u8);

impl Color {
    pub const fn empty() -> Color {
        Color(0)
    }

    pub const fn black() -> Color {
        Color(1)
    }

    pub const fn white() -> Color {
        Color(2)
    }

    pub const fn is_empty(&self) -> bool {
        self.0 == 0
    }
}

impl Default for Color {
    fn default() -> Self {
        Color::empty()
    }
}

#[derive(Debug, Clone, PartialEq)]
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

type Point = (u32, u32);

impl<T: Copy + Default + Hash> Board<T> {
    fn empty(width: u32, height: u32) -> Self {
        Board {
            width,
            height,
            points: vec![T::default(); (width * height) as usize],
        }
    }

    fn point_within(&self, (x, y): Point) -> bool {
        (0..self.width).contains(&x) && (0..self.height).contains(&y)
    }

    fn get_point(&self, (x, y): Point) -> T {
        self.points[(y * self.width + x) as usize]
    }

    fn point_mut(&mut self, (x, y): Point) -> &mut T {
        &mut self.points[(y * self.width + x) as usize]
    }

    fn idx_to_coord(&self, idx: usize) -> Option<Point> {
        if idx < self.points.len() {
            Some((idx as u32 % self.width, idx as u32 / self.width))
        } else {
            None
        }
    }

    fn surrounding_points(&self, p: Point) -> impl Iterator<Item = Point> {
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

    fn hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        Hash::hash(&self, &mut hasher);
        hasher.finish()
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Group {
    pub points: Vec<Point>,
    pub liberties: i32,
    pub team: Color,
    pub alive: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayState {
    // TODO: use smallvec?
    pub players_passed: Vec<bool>,
    pub last_stone: Option<Vec<(u32, u32)>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringState {
    pub groups: Vec<Group>,
    /// Vector of the board, marking who owns a point
    pub points: Board,
    pub scores: Vec<i32>,
    // TODO: use smallvec?
    pub players_accepted: Vec<bool>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GameState {
    Play(PlayState),
    Scoring(ScoringState),
    Done(ScoringState),
}

impl GameState {
    fn play(seat_count: usize) -> Self {
        GameState::Play(PlayState {
            players_passed: vec![false; seat_count],
            last_stone: None,
        })
    }

    fn scoring(board: &Board, seat_count: usize, scores: &[i32]) -> Self {
        let groups = find_groups(board);
        let points = score_board(board.width, board.height, &groups);
        let mut scores = scores.to_vec();
        for color in &points.points {
            if !color.is_empty() {
                scores[color.0 as usize - 1] += 2;
            }
        }
        GameState::Scoring(ScoringState {
            groups,
            points,
            scores,
            players_accepted: vec![false; seat_count],
        })
    }

    pub fn assume_play_mut(&mut self) -> &mut PlayState {
        match self {
            GameState::Play(state) => state,
            _ => panic!("Assumed play state but was in {:?}", self),
        }
    }

    pub fn assume_scoring_mut(&mut self) -> &mut ScoringState {
        match self {
            GameState::Scoring(state) => state,
            _ => panic!("Assumed scoring state but was in {:?}", self),
        }
    }
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub state: GameState,
    pub state_stack: Vec<GameState>,
    // TODO: use smallvec?
    pub seats: Vec<Seat>,
    pub points: Vec<i32>,
    pub turn: usize,
    pub pass_count: usize,
    pub board: Board,
    pub board_history: Vec<(u64, Board, GameState, Vec<i32>)>,
    /// Optimization for superko
    pub capture_count: usize,
    pub komis: Vec<i32>,
    pub mods: GameModifier,
    pub actions: Vec<GameAction>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum TakeSeatError {
    DoesNotExist,
    NotOpen,
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

#[derive(Debug, Clone, PartialEq)]
pub struct GameView {
    // TODO: we need a separate state view once we have hidden information
    pub state: GameState,
    pub seats: Vec<Seat>,
    pub turn: u32,
    pub board: Vec<Color>,
    pub size: (u8, u8),
    pub mods: GameModifier,
    pub points: Vec<i32>,
}

#[derive(Serialize, Deserialize)]
struct GameReplay {
    actions: Vec<GameAction>,
    mods: GameModifier,
    komis: Vec<i32>,
    seats: Vec<u8>,
    size: (u8, u8),
}

impl Game {
    pub fn standard(
        seats: &[u8],
        komis: Vec<i32>,
        size: (u8, u8),
        mods: GameModifier,
    ) -> Option<Game> {
        if !seats.iter().all(|&t| t > 0 && t <= 3) {
            return None;
        }

        if !(1..=4).contains(&seats.len()) || !(1..=3).contains(&komis.len()) {
            return None;
        }

        // Don't allow huge boards
        if size.0 > 19 || size.1 > 19 {
            return None;
        }

        let state = GameState::play(seats.len());
        let board = Board::empty(size.0 as _, size.1 as _);

        Some(Game {
            state: state.clone(),
            state_stack: Vec::new(),
            seats: seats.into_iter().map(|&t| Seat::new(Color(t))).collect(),
            points: komis.clone(),
            turn: 0,
            pass_count: 0,
            board: board.clone(),
            board_history: vec![(board.hash(), board, state, komis.clone())],
            capture_count: 0,
            komis,
            mods,
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
        let replay = GameReplay {
            actions: self.actions.clone(),
            komis: self.komis.clone(),
            size: (self.board.width as _, self.board.height as _),
            seats: self.seats.iter().map(|x| x.team.0).collect(),
            mods: self.mods.clone(),
        };

        let mut vec = Vec::new();
        replay
            .serialize(&mut serde_cbor::Serializer::new(&mut vec).packed_format())
            .expect("Game dump failed");
        vec
    }

    pub fn take_seat(&mut self, player_id: u64, seat_id: usize) -> Result<(), TakeSeatError> {
        let seat = self
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
        let seat = self
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
        if !self.seats.iter().any(|s| s.player == Some(player_id)) {
            return Err(MakeActionError::NotPlayer);
        }

        let res = match self.state {
            GameState::Play(_) => self.make_action_play(player_id, action.clone()),
            GameState::Scoring(_) => self.make_action_scoring(player_id, action.clone()),
            GameState::Done(_) => Err(MakeActionError::GameDone),
        };

        if res.is_ok() {
            self.actions.push(GameAction::play(player_id, action));
        }

        res
    }

    pub fn make_action_play(
        &mut self,
        player_id: u64,
        action: ActionKind,
    ) -> Result<(), MakeActionError> {
        let active_seat = self.seats.get(self.turn).expect("Game turn number invalid");
        if active_seat.player != Some(player_id) {
            return Err(MakeActionError::NotTurn);
        }
        match action {
            ActionKind::Place(x, y) => {
                let mut points_played = vec![];

                if self.mods.pixel {
                    // In pixel mode coordinate 0,0 is outside the board.
                    // This is to adjust for it.

                    if x > self.board.width || y > self.board.height {
                        return Err(MakeActionError::OutOfBounds);
                    }
                    let x = x as i32 - 1;
                    let y = y as i32 - 1;

                    let mut any_placed = false;
                    for &(x, y) in &[(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                        if x < 0 || y < 0 {
                            continue;
                        }
                        let coord = (x as u32, y as u32);
                        if !self.board.point_within(coord) {
                            continue;
                        }

                        let point = self.board.point_mut(coord);
                        if !point.is_empty() {
                            continue;
                        }
                        *point = active_seat.team;
                        points_played.push(coord);
                        any_placed = true;
                    }
                    if !any_placed {
                        return Err(MakeActionError::PointOccupied);
                    }
                } else {
                    if !self.board.point_within((x, y)) {
                        return Err(MakeActionError::OutOfBounds);
                    }

                    // TODO: don't repeat yourself
                    let point = self.board.point_mut((x, y));
                    if !point.is_empty() {
                        return Err(MakeActionError::PointOccupied);
                    }

                    *point = active_seat.team;
                    points_played.push((x, y));
                }

                let groups = find_groups(&self.board);
                let dead = groups.iter().filter(|g| g.liberties == 0);
                let opp_died = dead.clone().any(|g| g.team != active_seat.team);

                let mut captures = 0;

                for group in dead {
                    // If the opponent died, suicide is ignored
                    if opp_died && group.team == active_seat.team {
                        continue;
                    }

                    // Suicide is illegal, bail out
                    if !opp_died {
                        // TODO: don't repeat yourself
                        self.board = self
                            .board_history
                            .last()
                            .expect("board_history.last() shouldn't be None")
                            .1
                            .clone();
                        return Err(MakeActionError::Suicide);
                    }

                    for &point in &group.points {
                        captures += 1;
                        *self.board.point_mut(point) = Color::empty();
                    }

                    if let Some(ponnuki) = self.mods.ponnuki_is_points {
                        if group.points.len() == 1 && group.team != active_seat.team {
                            let p = group.points[0];
                            let neighbours = self.board.surrounding_points(p).collect::<Vec<_>>();
                            if neighbours.len() == 4
                                && neighbours
                                    .iter()
                                    .all(|x| self.board.get_point(*x) == active_seat.team)
                            {
                                self.points[(active_seat.team.0 - 1) as usize] += ponnuki;
                            }
                        }
                    }
                }

                let hash = self.board.hash();

                // Superko
                // We only need to scan back capture_count boards, as per Ten 1p's clever idea.
                // The board can't possibly repeat further back than the number of removed stones.
                for (old_hash, old_board, _, _) in self
                    .board_history
                    .iter()
                    .rev()
                    .take(self.capture_count + captures)
                {
                    if *old_hash == hash && old_board == &self.board {
                        self.board = self
                            .board_history
                            .last()
                            .expect("board_history.last() shouldn't be None")
                            .1
                            .clone();
                        return Err(MakeActionError::Ko);
                    }
                }

                self.turn += 1;
                if self.turn >= self.seats.len() {
                    self.turn = 0;
                }

                let state = self.state.assume_play_mut();
                state.last_stone = Some(points_played);
                for passed in &mut state.players_passed {
                    *passed = false;
                }

                self.board_history.push((
                    hash,
                    self.board.clone(),
                    self.state.clone(),
                    self.points.clone(),
                ));
                self.capture_count += captures;
            }
            ActionKind::Pass => {
                let seat_idx = self.turn;
                let state = self.state.assume_play_mut();
                state.players_passed[seat_idx] = true;
                state.last_stone = None;

                if state.players_passed.iter().all(|x| *x) {
                    for passed in &mut state.players_passed {
                        *passed = false;
                    }
                    let old_state = std::mem::replace(
                        &mut self.state,
                        GameState::scoring(&self.board, self.seats.len(), &self.points),
                    );
                    self.state_stack.push(old_state);
                }

                self.turn += 1;
                if self.turn >= self.seats.len() {
                    self.turn = 0;
                }

                self.board_history.push((
                    self.board.hash(),
                    self.board.clone(),
                    self.state.clone(),
                    self.points.clone(),
                ));
            }
            ActionKind::Cancel => {
                // Undo a turn
                if self.board_history.len() < 2 {
                    return Err(MakeActionError::OutOfBounds);
                }

                self.board_history
                    .pop()
                    .ok_or(MakeActionError::OutOfBounds)?;
                let (_, last_board, last_state, last_points) = self
                    .board_history
                    .last()
                    .ok_or(MakeActionError::OutOfBounds)?;
                self.board = last_board.clone();
                self.state = last_state.clone();
                self.points = last_points.clone();
                self.turn = if self.turn == 0 {
                    self.seats.len() - 1
                } else {
                    self.turn - 1
                };
            }
            unknown => {
                println!("Play state got unexpected action {:?}", unknown);
            }
        }

        Ok(())
    }

    pub fn make_action_scoring(
        &mut self,
        player_id: u64,
        action: ActionKind,
    ) -> Result<(), MakeActionError> {
        match action {
            ActionKind::Place(x, y) => {
                let state = self.state.assume_scoring_mut();

                let group = state.groups.iter_mut().find(|g| g.points.contains(&(x, y)));

                let group = match group {
                    Some(g) => g,
                    None => return Ok(()),
                };

                group.alive = !group.alive;

                state.points = score_board(self.board.width, self.board.height, &state.groups);
                state.scores = self.points.clone();
                for color in &state.points.points {
                    if !color.is_empty() {
                        state.scores[color.0 as usize - 1] += 2;
                    }
                }

                for accept in &mut state.players_accepted {
                    *accept = false;
                }
            }
            ActionKind::Pass => {
                // A single player can hold multiple seats so we have to mark every seat they hold
                let seats = self
                    .seats
                    .iter()
                    .enumerate()
                    .filter(|x| x.1.player == Some(player_id));

                let state = self.state.assume_scoring_mut();
                for (seat_idx, _) in seats {
                    state.players_accepted[seat_idx] = true;
                }
                if state.players_accepted.iter().all(|x| *x) {
                    self.state = GameState::Done(state.clone());
                }
            }
            ActionKind::Cancel => {
                self.state = self
                    .state_stack
                    .pop()
                    .expect("Empty state stack in scoring cancel");
            }
        }

        Ok(())
    }

    pub fn get_view(&self) -> GameView {
        GameView {
            state: self.state.clone(),
            seats: self.seats.clone(),
            turn: self.turn as _,
            board: self.board.points.clone(),
            size: (self.board.width as u8, self.board.height as u8),
            mods: self.mods.clone(),
            points: self.points.clone(),
        }
    }
}

fn find_groups(board: &Board) -> Vec<Group> {
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
    let mut stack = Vec::new();
    let mut groups = Vec::new();

    while let Some(point) = legal_points.pop() {
        let mut group = Group::default();
        group.alive = true;
        group.team = board.get_point(point);
        if group.team.is_empty() {
            unreachable!("scanned an empty point");
        }

        stack.push(point);

        // TODO: change stack to VecDeque so we can pop_left .. more efficient walk
        while let Some(point) = stack.pop() {
            group.points.push(point);
            for point in board.surrounding_points(point) {
                if !seen.insert(point) {
                    continue;
                }

                match board.get_point(point) {
                    x if x == group.team => {
                        stack.push(point);
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

/// Scores a board by filling in fully surrounded empty spaces based on chinese rules
fn score_board(width: u32, height: u32, groups: &[Group]) -> Board {
    let mut board = Board::empty(width, height);

    // Fill living groups to the board
    for group in groups {
        if !group.alive {
            continue;
        }
        for point in &group.points {
            *board.point_mut(*point) = group.team;
        }
    }

    // Find empty points
    let mut legal_points = board
        .points
        .iter()
        .enumerate()
        .filter_map(|(idx, c)| {
            if c.is_empty() {
                board.idx_to_coord(idx)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();

    #[derive(Copy, Clone)]
    enum SeenTeams {
        Zero,
        One(Color),
        Many,
    }
    use SeenTeams::*;

    let mut seen = HashSet::new();
    let mut stack = VecDeque::new();
    let mut marked = Vec::new();

    while let Some(point) = legal_points.pop() {
        stack.push_back(point);

        let mut collisions = SeenTeams::Zero;

        while let Some(point) = stack.pop_front() {
            marked.push(point);
            for point in board.surrounding_points(point) {
                if !seen.insert(point) {
                    continue;
                }

                match board.get_point(point) {
                    Color(0) => {
                        stack.push_back(point);
                        legal_points.retain(|x| *x != point);
                    }
                    c => {
                        collisions = match collisions {
                            Zero => One(c),
                            One(x) if x == c => One(x),
                            One(_) => Many,
                            Many => Many,
                        }
                    }
                }
            }
        }

        // The floodfill touched only a single color -> this must be their territory
        if let One(color) = collisions {
            for point in marked.drain(..) {
                *board.point_mut(point) = color;
            }
        }

        seen.clear();
        marked.clear();
    }

    board
}
