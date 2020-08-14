use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
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

    fn black() -> Seat {
        Seat::new(Color::black())
    }

    fn white() -> Seat {
        Seat::new(Color::white())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Place(u32, u32),
    Pass,
    Cancel,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameAction {
    pub seat: usize,
    pub action: ActionKind,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Board<T = Color> {
    pub width: u32,
    pub height: u32,
    pub points: Vec<T>,
}

type Point = (u32, u32);

impl<T: Copy + Default> Board<T> {
    fn empty(width: u32, height: u32) -> Self {
        Board {
            width,
            height,
            points: vec![T::default(); (width * height) as usize],
        }
    }

    fn point_within(&self, (x, y): Point) -> bool {
        !(0..self.width).contains(&x) || !(0..self.height).contains(&y)
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
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScoringState {
    pub groups: Vec<Group>,
    /// Vector of the board, marking who owns a point
    pub points: Board,
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
        })
    }

    fn scoring(board: &Board, seat_count: usize) -> Self {
        let groups = find_groups(board);
        let points = score_board(board.width, board.height, &groups);
        GameState::Scoring(ScoringState {
            groups,
            points,
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

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub state: GameState,
    // TODO: use smallvec?
    pub seats: Vec<Seat>,
    pub turn: usize,
    pub pass_count: usize,
    pub board: Board,
    pub ko_point: Option<Point>,
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
}

impl Game {
    pub fn standard() -> Game {
        Game {
            state: GameState::play(2),
            seats: vec![Seat::black(), Seat::white()],
            turn: 0,
            pass_count: 0,
            board: Board::empty(19, 19),
            ko_point: None,
        }
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

        match self.state {
            GameState::Play(_) => self.make_action_play(player_id, action),
            GameState::Scoring(_) => self.make_action_scoring(player_id, action),
            GameState::Done(_) => Err(MakeActionError::GameDone),
        }
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
                if self.board.point_within((x, y)) {
                    return Err(MakeActionError::OutOfBounds);
                }

                let point = self.board.point_mut((x, y));
                if !point.is_empty() {
                    return Err(MakeActionError::PointOccupied);
                }

                if self.ko_point == Some((x, y)) {
                    return Err(MakeActionError::Ko);
                }

                *point = active_seat.team;

                let groups = find_groups(&self.board);
                let dead = groups.iter().filter(|g| g.liberties == 0);
                let opp_died = dead.clone().any(|g| g.team != active_seat.team);
                let mut dead_count = 0;

                let mut ko_point = None;

                for group in dead {
                    // If the opponent died, suicide is ignored
                    if opp_died && group.team == active_seat.team {
                        continue;
                    }

                    // Suicide is illegal, bail out
                    if !opp_died {
                        *self.board.point_mut((x, y)) = Color::empty();
                        return Err(MakeActionError::Suicide);
                    }

                    dead_count += 1;
                    if dead_count == 1 && group.points.len() == 1 {
                        ko_point = Some(group.points[0]);
                    }

                    for &point in &group.points {
                        *self.board.point_mut(point) = Color::empty();
                    }
                }

                self.ko_point = if dead_count == 1 { ko_point } else { None };

                self.turn += 1;
                if self.turn >= self.seats.len() {
                    self.turn = 0;
                }

                let state = self.state.assume_play_mut();
                for passed in &mut state.players_passed {
                    *passed = false;
                }
            }
            ActionKind::Pass => {
                let seat_idx = self.turn;
                let state = self.state.assume_play_mut();
                state.players_passed[seat_idx] = true;

                if state.players_passed.iter().all(|x| *x) {
                    self.state = GameState::scoring(&self.board, self.seats.len());
                }

                self.turn += 1;
                if self.turn >= self.seats.len() {
                    self.turn = 0;
                }
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
                self.state = GameState::play(self.seats.len());
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
