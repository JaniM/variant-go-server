
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
pub struct Game {
    pub seats: Vec<Seat>,
    pub turn: usize,
    pub board: Vec<Option<Color>>
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
            board: vec![None; 19*19],
        }
    }

    pub fn take_seat(&mut self, player_id: u64, seat_id: usize) -> Result<(), TakeSeatError> {
        let seat = self.seats.get_mut(seat_id).ok_or(TakeSeatError::DoesNotExist)?;
        if seat.player.is_some() {
            return Err(TakeSeatError::NotOpen);
        }
        seat.player = Some(player_id);
        Ok(())
    }

    pub fn leave_seat(&mut self, player_id: u64, seat_id: usize) -> Result<(), TakeSeatError> {
        let seat = self.seats.get_mut(seat_id).ok_or(TakeSeatError::DoesNotExist)?;
        if seat.player != Some(player_id) {
            return Err(TakeSeatError::NotOpen);
        }
        seat.player = None;
        Ok(())
    }

    pub fn make_action(&mut self, player_id: u64, action: ActionKind) -> Result<(), MakeActionError> {
        let active_seat = self.seats.get(self.turn).expect("Game turn number invalid");
        if active_seat.player != Some(player_id) {
            return Err(MakeActionError::NotTurn);
        }

        match action {
            ActionKind::Place(x, y) => {
                if !(0..19).contains(&x) || !(0..19).contains(&y) {
                    return Err(MakeActionError::OutOfBounds);
                }

                let idx = (y * 19 + x) as usize;
                if self.board[idx] != None {
                    return Err(MakeActionError::PointOccupied);
                }

                self.board[idx] = Some(active_seat.team);
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
            board: self.board.clone(),
        }
    }
}

