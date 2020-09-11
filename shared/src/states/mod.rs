pub mod free_placement;
pub mod play;
pub mod scoring;

pub use self::free_placement::FreePlacement;
pub use self::play::PlayState;
pub use self::scoring::ScoringState;

use serde::{Deserialize, Serialize};
use crate::assume::AssumeFrom;
use crate::game::Board;

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
