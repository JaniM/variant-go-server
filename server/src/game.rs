use std::collections::HashSet;

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Black,
    White,
}

impl Default for Color {
    fn default() -> Self {
        Color::Black
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
        Seat::new(Color::Black)
    }

    fn white() -> Seat {
        Seat::new(Color::White)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ActionKind {
    Place(u32, u32),
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameAction {
    pub seat: usize,
    pub action: ActionKind,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Board {
    pub width: u32,
    pub height: u32,
    pub points: Vec<Option<Color>>,
}

type Point = (u32, u32);

impl Board {
    fn empty(width: u32, height: u32) -> Board {
        Board {
            width,
            height,
            points: vec![None; (width * height) as usize],
        }
    }

    fn point_within(&self, (x, y): Point) -> bool {
        !(0..self.width).contains(&x) || !(0..self.height).contains(&y)
    }

    fn get_point(&self, (x, y): Point) -> Option<Color> {
        self.points[(y * self.width + x) as usize]
    }

    fn point_mut(&mut self, (x, y): Point) -> &mut Option<Color> {
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

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub seats: Vec<Seat>,
    pub turn: usize,
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct GameView {
    pub seats: Vec<Seat>,
    pub turn: u32,
    pub board: Vec<Option<Color>>,
}

impl Game {
    pub fn standard() -> Game {
        Game {
            seats: vec![Seat::black(), Seat::white()],
            turn: 0,
            board: Board {
                width: 19,
                height: 19,
                points: vec![None; 19 * 19],
            },
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
                if *point != None {
                    return Err(MakeActionError::PointOccupied);
                }

                if self.ko_point == Some((x, y)) {
                    return Err(MakeActionError::Ko);
                }

                *point = Some(active_seat.team);

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
                        *self.board.point_mut((x, y)) = None;
                        return Err(MakeActionError::Suicide);
                    }

                    dead_count += 1;
                    if dead_count == 1 && group.points.len() == 1 {
                        ko_point = Some(group.points[0]);
                    }

                    for &point in &group.points {
                        *self.board.point_mut(point) = None;
                    }
                }

                self.ko_point = if dead_count == 1 { ko_point } else { None };

                self.turn += 1;
                if self.turn >= self.seats.len() {
                    self.turn = 0;
                }
            }
        }

        Ok(())
    }

    pub fn get_view(&self) -> GameView {
        GameView {
            seats: self.seats.clone(),
            turn: self.turn as _,
            board: self.board.points.clone(),
        }
    }
}

#[derive(Default)]
pub struct Group {
    pub points: Vec<Point>,
    pub liberties: i32,
    pub team: Color,
}

fn find_groups(board: &Board) -> Vec<Group> {
    let mut legal_points = board
        .points
        .iter()
        .enumerate()
        .filter_map(|(idx, c)| c.and_then(|_| board.idx_to_coord(idx)))
        .collect::<Vec<_>>();

    let mut seen = HashSet::new();
    let mut stack = Vec::new();
    let mut groups = Vec::new();

    while let Some(point) = legal_points.pop() {
        let mut group = Group::default();
        group.team = board.get_point(point).expect("scanned an empty point");

        stack.push(point);

        while let Some(point) = stack.pop() {
            group.points.push(point);
            for point in board.surrounding_points(point) {
                if !seen.insert(point) {
                    continue;
                }

                match board.get_point(point) {
                    Some(x) if x == group.team => {
                        stack.push(point);
                        legal_points.retain(|x| *x != point);
                    }
                    None => group.liberties += 1,
                    _ => {}
                }
            }
        }

        seen.clear();
        groups.push(group);
    }

    groups
}
