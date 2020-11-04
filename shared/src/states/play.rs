mod n_plus_one;
mod tetris;
pub(crate) mod traitor;

use crate::game::{
    find_groups, ActionChange, ActionKind, Board, BoardHistory, Color, GameState, Group, GroupVec,
    MakeActionError, MakeActionResult, Point, SharedState, VisibilityBoard,
};
use serde::{Deserialize, Serialize};

use bitmaps::Bitmap;
use tinyvec::tiny_vec;

use super::ScoringState;

type Revealed = bool;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlayState {
    // TODO: use smallvec?
    pub players_passed: Vec<bool>,
    pub last_stone: Option<GroupVec<(u32, u32)>>,
    /// Optimization for superko
    pub capture_count: usize,
}

impl PlayState {
    pub fn new(seat_count: usize) -> Self {
        PlayState {
            players_passed: vec![false; seat_count],
            last_stone: None,
            capture_count: 0,
        }
    }

    fn place_stone(
        &mut self,
        shared: &mut SharedState,
        (x, y): Point,
        color_placed: Color,
    ) -> MakeActionResult<GroupVec<Point>> {
        let mut points_played = GroupVec::new();

        if shared.mods.pixel {
            // In pixel mode coordinate 0,0 is outside the board.
            // This is to adjust for it.

            if x > shared.board.width || y > shared.board.height {
                return Err(MakeActionError::OutOfBounds);
            }
            let x = x as i32 - 1;
            let y = y as i32 - 1;

            let mut any_placed = false;
            let mut any_revealed = false;
            for &(x, y) in &[(x, y), (x + 1, y), (x, y + 1), (x + 1, y + 1)] {
                let coord = match shared.board.wrap_point(x, y) {
                    Some(x) => x,
                    None => continue,
                };

                let point = shared.board.point_mut(coord);
                if let Some(visibility) = &mut shared.board_visibility {
                    if !visibility.get_point(coord).is_empty() {
                        any_revealed = true;
                        points_played.push(coord);
                    }
                    *visibility.point_mut(coord) = Bitmap::new();
                }
                if !point.is_empty() {
                    continue;
                }
                *point = color_placed;
                points_played.push(coord);
                any_placed = true;
            }
            if !any_placed {
                if any_revealed {
                    self.last_stone = Some(points_played);
                    return Ok(GroupVec::new());
                }
                return Err(MakeActionError::PointOccupied);
            }
        } else {
            if !shared.board.point_within((x, y)) {
                return Err(MakeActionError::OutOfBounds);
            }

            // TODO: don't repeat yourself
            let point = shared.board.point_mut((x, y));
            let revealed = if let Some(visibility) = &mut shared.board_visibility {
                let revealed = !visibility.get_point((x, y)).is_empty();
                *visibility.point_mut((x, y)) = Bitmap::new();
                revealed
            } else {
                false
            };
            if !point.is_empty() {
                if revealed {
                    self.last_stone = Some(tiny_vec![[Point; 8] => (x, y)]);
                    return Ok(points_played);
                }
                return Err(MakeActionError::PointOccupied);
            }

            *point = color_placed;
            points_played.push((x, y));
        }

        Ok(points_played)
    }

    fn capture(
        &self,
        shared: &mut SharedState,
        points_played: &mut GroupVec<Point>,
    ) -> (usize, Revealed) {
        let active_seat = shared.get_active_seat();
        let mut captures = 0;
        let mut revealed = false;

        if shared.mods.phantom.is_some() {
            let groups = find_groups(&shared.board);
            let ataris = groups.iter().filter(|g| g.liberties == 1);
            for group in ataris {
                let reveals = reveal_group(shared.board_visibility.as_mut(), group, &shared.board);
                revealed = revealed || reveals;
            }
        }

        let mut kill = |shared: &mut SharedState, group: &Group| -> Revealed {
            let board = &mut shared.board;
            for point in &group.points {
                *board.point_mut(*point) = Color::empty();
                captures += 1;
            }
            let reveals = reveal_group(shared.board_visibility.as_mut(), group, board);

            if let Some(ponnuki) = shared.mods.ponnuki_is_points {
                let surrounding_count = board.surrounding_points(group.points[0]).count();
                if group.points.len() == 1
                    && surrounding_count == 4
                    && board
                        .surrounding_points(group.points[0])
                        .all(|p| board.get_point(p) == active_seat.team)
                    && board
                        .surrounding_diagonal_points(group.points[0])
                        .all(|p| board.get_point(p) != active_seat.team)
                {
                    shared.points[active_seat.team.0 as usize - 1] += ponnuki;
                }
            }

            reveals
        };

        let groups = find_groups(&shared.board);
        let dead_opponents = groups
            .iter()
            .filter(|g| g.liberties == 0 && g.team != active_seat.team);

        for group in dead_opponents {
            // Don't forget about short-circuiting boolean operators...
            let reveals = kill(shared, group);
            revealed = revealed || reveals;
        }

        // TODO: only re-scan own previously dead grouos
        let groups = find_groups(&shared.board);
        let dead_own = groups
            .iter()
            .filter(|g| g.liberties == 0 && g.team == active_seat.team);

        for group in dead_own {
            let mut removed_move = false;
            for point in &group.points {
                if points_played.contains(point) {
                    points_played.retain(|x| x != point);
                    *shared.board.point_mut(*point) = Color::empty();
                    removed_move = true;
                }
            }
            let reveals = reveal_group(shared.board_visibility.as_mut(), group, &shared.board);
            revealed = revealed || reveals;

            // If no illegal move has been made (eg. we suicided with a traitor stone), kill the group.
            if !removed_move {
                revealed = revealed || kill(shared, group);
            }
        }

        if shared.mods.captures_give_points.is_some() {
            shared.points[active_seat.team.0 as usize - 1] += captures as i32 * 2;
        }

        (captures, revealed)
    }

    /// Superko
    /// We only need to scan back capture_count boards, as per Ten 1p's clever idea.
    /// The board can't possibly repeat further back than the number of removed stones.
    fn superko(
        &self,
        shared: &mut SharedState,
        captures: usize,
        hash: u64,
    ) -> MakeActionResult<()> {
        for BoardHistory {
            hash: old_hash,
            board: old_board,
            ..
        } in shared
            .board_history
            .iter()
            .rev()
            .take(self.capture_count + captures)
        {
            if *old_hash == hash && old_board == &shared.board {
                let BoardHistory {
                    board: old_board,
                    points: old_points,
                    ..
                } = shared
                    .board_history
                    .last()
                    .expect("board_history.last() shouldn't be None")
                    .clone();
                shared.board = old_board;
                shared.points = old_points;
                return Err(MakeActionError::Ko);
            }
        }

        Ok(())
    }

    fn make_action_place(
        &mut self,
        shared: &mut SharedState,
        (x, y): (u32, u32),
        color_placed: Color,
    ) -> MakeActionResult {
        // TODO: should use some kind of set to make suicide prevention faster
        let mut points_played = self.place_stone(shared, (x, y), color_placed)?;
        if points_played.is_empty() {
            return Ok(ActionChange::None);
        }

        if let Some(rule) = &shared.mods.tetris {
            // This is valid because points_played is empty if the move is illegal.
            use tetris::TetrisResult::*;
            match tetris::check(&mut points_played, &mut shared.board, rule) {
                Nothing => {}
                Illegal => {
                    return Err(MakeActionError::Illegal);
                }
            }
        }

        if shared.mods.phantom.is_some() {
            let seat = shared.get_active_seat();
            let visibility = shared
                .board_visibility
                .as_mut()
                .expect("Visibility board not initialized with phantom go");
            for &point in &points_played {
                // The hidden layer can't deal with being able to see someone else's stones, so if we played
                // a stone of wrong color (eg. a traitor), just reveal it.
                if shared.board.get_point(point) != seat.team {
                    continue;
                }

                let mut v = Bitmap::new();
                v.set(seat.team.as_usize(), true);

                *visibility.point_mut(point) = v;
            }
        }

        let (captures, revealed) = self.capture(shared, &mut points_played);

        if points_played.is_empty() {
            let BoardHistory { board, points, .. } = shared
                .board_history
                .last()
                .expect("board_history.last() shouldn't be None")
                .clone();
            shared.board = board;
            shared.points = points;

            if revealed {
                return Ok(ActionChange::None);
            }
            return Err(MakeActionError::Suicide);
        }

        let hash = shared.board.hash();

        self.superko(shared, captures, hash)?;

        let new_turn = if let Some(rule) = &shared.mods.n_plus_one {
            use n_plus_one::NPlusOneResult::*;
            match n_plus_one::check(
                &points_played,
                &shared.board,
                shared.board_visibility.as_mut(),
                rule,
            ) {
                ExtraTurn => true,
                Nothing => false,
            }
        } else {
            false
        };

        self.last_stone = Some(points_played);

        // TODO: Handle this at the view layer instead to have the marker visible for your own stones.
        if shared.mods.phantom.is_some() {
            self.last_stone = None;
        }

        for passed in &mut self.players_passed {
            *passed = false;
        }

        self.next_turn(shared, new_turn);
        self.capture_count += captures;

        Ok(ActionChange::None)
    }

    fn make_action_pass(&mut self, shared: &mut SharedState) -> MakeActionResult {
        let active_seat = shared.get_active_seat();

        for (seat, passed) in shared.seats.iter().zip(self.players_passed.iter_mut()) {
            if seat.team == active_seat.team {
                *passed = true;
            }
        }

        self.next_turn(shared, false);

        if shared
            .seats
            .iter()
            .zip(&self.players_passed)
            .all(|(s, &pass)| s.resigned || pass)
        {
            for passed in &mut self.players_passed {
                *passed = false;
            }
            return Ok(ActionChange::PushState(GameState::scoring(
                &shared.board,
                &shared.seats,
                &shared.points,
            )));
        }

        Ok(ActionChange::None)
    }

    fn make_action_cancel(&mut self, shared: &mut SharedState) -> MakeActionResult {
        // Undo a turn
        if shared.board_history.len() < 2 {
            return Err(MakeActionError::OutOfBounds);
        }

        self.rollback_turn(shared, true)
    }

    fn rollback_turn(
        &mut self,
        shared: &mut SharedState,
        roll_visibility: bool,
    ) -> MakeActionResult {
        shared
            .board_history
            .pop()
            .ok_or(MakeActionError::OutOfBounds)?;
        let history = shared
            .board_history
            .last()
            .ok_or(MakeActionError::OutOfBounds)?;

        shared.board = history.board.clone();
        if roll_visibility {
            shared.board_visibility = history.board_visibility.clone();
        }
        shared.points = history.points.clone();
        shared.turn = history.turn;
        shared.traitor = history.traitor.clone();

        *self = history.state.assume::<PlayState>().clone();

        Ok(ActionChange::None)
    }

    fn make_action_resign(&mut self, shared: &mut SharedState) -> MakeActionResult {
        let active_seat = shared
            .seats
            .get_mut(shared.turn)
            .expect("Game turn number invalid");

        active_seat.resigned = true;

        if shared.seats.iter().filter(|s| !s.resigned).count() <= 1 {
            return Ok(ActionChange::PushState(GameState::Done(ScoringState::new(
                &shared.board,
                &shared.seats,
                &shared.points,
            ))));
        }

        loop {
            shared.turn += 1;
            if shared.turn >= shared.seats.len() {
                shared.turn = 0;
            }
            if !shared.get_active_seat().resigned {
                break;
            }
        }

        Ok(ActionChange::None)
    }

    pub fn make_action(
        &mut self,
        shared: &mut SharedState,
        player_id: u64,
        action: ActionKind,
    ) -> MakeActionResult {
        let active_seat = shared.get_active_seat();
        if active_seat.player != Some(player_id) {
            return Err(MakeActionError::NotTurn);
        }

        let res = match action {
            ActionKind::Place(x, y) => {
                let depth = shared.board_history.len();

                let res = self.make_action_place(shared, (x, y), active_seat.team);

                if res.is_ok() && shared.board_history.len() > depth && shared.traitor.is_some() {
                    // Depth increased -> the move is legal.
                    // Replay using traitor stone.

                    let _ = self.rollback_turn(shared, false);

                    let traitor = shared.traitor.clone();
                    let color_placed = if let Some(state) = &mut shared.traitor {
                        state.next_color(active_seat.team)
                    } else {
                        unreachable!();
                    };

                    let res = self.make_action_place(shared, (x, y), color_placed);

                    if res.is_err() {
                        shared.traitor = traitor;
                    }
                    res
                } else {
                    res
                }
            }
            ActionKind::Pass => self.make_action_pass(shared),
            ActionKind::Cancel => self.make_action_cancel(shared),
            ActionKind::Resign => self.make_action_resign(shared),
        };

        let res = res?;

        self.set_zen_teams(shared);

        Ok(res)
    }

    fn next_turn(&mut self, shared: &mut SharedState, new_turn: bool) {
        if !new_turn {
            loop {
                shared.turn += 1;
                if shared.turn >= shared.seats.len() {
                    shared.turn = 0;
                }
                if !shared.get_active_seat().resigned {
                    break;
                }
            }
        }

        shared.board_history.push(BoardHistory {
            hash: shared.board.hash(),
            board: shared.board.clone(),
            board_visibility: shared.board_visibility.clone(),
            state: GameState::Play(self.clone()),
            points: shared.points.clone(),
            turn: shared.turn,
            traitor: shared.traitor.clone(),
        });
    }

    fn set_zen_teams(&mut self, shared: &mut SharedState) {
        let move_number = shared.board_history.len() - 1;
        if let Some(zen) = &shared.mods.zen_go {
            for seat in &mut shared.seats {
                seat.team = Color((move_number % zen.color_count as usize) as u8 + 1);
            }
        }
    }
}

pub(self) fn reveal_group(
    visibility: Option<&mut VisibilityBoard>,
    group: &Group,
    board: &Board,
) -> Revealed {
    let mut revealed = false;

    if let Some(visibility) = visibility {
        for &point in &group.points {
            revealed = revealed || !visibility.get_point(point).is_empty();
            *visibility.point_mut(point) = Bitmap::new();
            for point in board.surrounding_points(point) {
                revealed = revealed || !visibility.get_point(point).is_empty();
                *visibility.point_mut(point) = Bitmap::new();
            }
        }
    }

    revealed
}
