use crate::game::{
    ActionChange, ActionKind, Board, BoardHistory, Color, GameState, MakeActionError,
    MakeActionResult, Seat, SharedState, VisibilityBoard,
};
use serde::{Deserialize, Serialize};

use itertools::izip;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FreePlacement {
    // One board per visibility group (= team or player)
    pub boards: Vec<Board>,
    pub stones_placed: Vec<u32>,
    pub players_ready: Vec<bool>,
    pub teams_share_stones: bool,
}

impl FreePlacement {
    pub fn new(
        seat_count: usize,
        team_count: usize,
        board: Board,
        teams_share_stones: bool,
    ) -> Self {
        let count = if teams_share_stones {
            team_count
        } else {
            seat_count
        };
        FreePlacement {
            boards: vec![board; count],
            stones_placed: vec![0; count],
            players_ready: vec![false; seat_count],
            teams_share_stones,
        }
    }

    fn make_action_place(
        &mut self,
        shared: &mut SharedState,
        player_id: u64,
        (x, y): (u32, u32),
    ) -> MakeActionResult {
        let (seat_idx, active_seat) = get_seat(&shared.seats, player_id);
        let team = active_seat.team;

        let board = if self.teams_share_stones {
            &mut self.boards[team.0 as usize - 1]
        } else {
            &mut self.boards[seat_idx]
        };
        let stones_placed = if self.teams_share_stones {
            &mut self.stones_placed[team.0 as usize - 1]
        } else {
            &mut self.stones_placed[seat_idx]
        };

        if *stones_placed >= shared.mods.hidden_move.as_ref().unwrap().placement_count {
            return Err(MakeActionError::PointOccupied);
        }

        if shared.mods.pixel {
            // In pixel mode coordinate 0,0 is outside the board.
            // This is to adjust for it.

            if x > board.width || y > board.height {
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
                if !board.point_within(coord) {
                    continue;
                }

                let point = board.point_mut(coord);
                if !point.is_empty() {
                    continue;
                }
                *point = active_seat.team;
                any_placed = true;
            }
            if !any_placed {
                return Err(MakeActionError::PointOccupied);
            }
        } else {
            if !board.point_within((x, y)) {
                return Err(MakeActionError::OutOfBounds);
            }

            // TODO: don't repeat yourself
            let point = board.point_mut((x, y));
            if !point.is_empty() {
                return Err(MakeActionError::PointOccupied);
            }

            *point = active_seat.team;
        }

        *stones_placed += 1;

        Ok(ActionChange::None)
    }

    pub fn make_action_pass(
        &mut self,
        shared: &mut SharedState,
        player_id: u64,
    ) -> MakeActionResult {
        let (seat_idx, _active_seat) = get_seat(&shared.seats, player_id);
        self.players_ready[seat_idx] = true;

        if self.players_ready.iter().all(|x| *x) {
            let (board, visibility) = self.build_board(shared.board.clone());

            shared.board = board;
            shared.board_visibility = Some(visibility);

            let state = GameState::play(shared.seats.len());

            shared.board_history = vec![BoardHistory {
                hash: shared.board.hash(),
                board: shared.board.clone(),
                board_visibility: shared.board_visibility.clone(),
                state: state.clone(),
                points: shared.points.clone(),
            }];

            return Ok(ActionChange::SwapState(state));
        }

        Ok(ActionChange::None)
    }

    fn build_board(&self, mut board: Board) -> (Board, VisibilityBoard) {
        let mut visibility = VisibilityBoard::empty(board.width, board.height);

        for view_board in &self.boards {
            for (a, b, v) in izip!(
                &mut board.points,
                &view_board.points,
                &mut visibility.points
            ) {
                if *b == Color::empty() {
                    continue;
                }

                v.set(b.as_usize(), true);

                // Double-committed points become empty!
                if v.len() == 1 {
                    *a = *b;
                } else {
                    *a = Color::empty();
                }
            }
        }

        (board, visibility)
    }

    pub fn make_action_cancel(
        &mut self,
        shared: &mut SharedState,
        player_id: u64,
    ) -> MakeActionResult {
        let (seat_idx, active_seat) = get_seat(&shared.seats, player_id);
        let team = active_seat.team;

        let board = if self.teams_share_stones {
            &mut self.boards[team.0 as usize - 1]
        } else {
            &mut self.boards[seat_idx]
        };
        let stones_placed = if self.teams_share_stones {
            &mut self.stones_placed[team.0 as usize - 1]
        } else {
            &mut self.stones_placed[seat_idx]
        };

        self.players_ready[seat_idx] = false;
        *board = shared.board.clone();
        *stones_placed = 0;

        Ok(ActionChange::None)
    }

    pub fn make_action(
        &mut self,
        shared: &mut SharedState,
        player_id: u64,
        action: ActionKind,
    ) -> MakeActionResult {
        match action {
            ActionKind::Place(x, y) => self.make_action_place(shared, player_id, (x, y)),
            ActionKind::Pass => self.make_action_pass(shared, player_id),
            ActionKind::Cancel => self.make_action_cancel(shared, player_id),
        }
    }
}

fn get_seat(seats: &[Seat], player_id: u64) -> (usize, &Seat) {
    // In free placement it is assumed a player can only hold a single seat.
    seats
        .iter()
        .enumerate()
        .find(|(_, x)| x.player == Some(player_id))
        .expect("User has no seat")
}
