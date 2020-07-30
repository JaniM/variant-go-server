#[derive(Debug, Copy, Clone, PartialEq)]
pub enum Color {
    Black,
    White,
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

impl Board {
    fn empty(width: u32, height: u32) -> Board {
        Board {
            width,
            height,
            points: vec![None; (width * height) as usize],
        }
    }

    fn point_within(&self, (x, y): (u32, u32)) -> bool {
        !(0..self.width).contains(&x) || !(0..self.height).contains(&y)
    }

    fn get_point(&self, (x, y): (u32, u32)) -> Option<Color> {
        self.points[(y * self.width + x) as usize]
    }

    fn point_mut(&mut self, (x, y): (u32, u32)) -> &mut Option<Color> {
        &mut self.points[(y * self.width + x) as usize]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Game {
    pub seats: Vec<Seat>,
    pub turn: usize,
    pub board: Board,
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

                *point = Some(active_seat.team);
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
