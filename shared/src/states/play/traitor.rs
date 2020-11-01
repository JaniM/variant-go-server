use rand::prelude::*;
use rand_pcg::Lcg64Xsh32;

use crate::game::Color;
use crate::game::GroupVec;
use crate::game::TraitorGo;

#[derive(Clone, Default)]
struct TeamState {
    traitor_count: u32,
    stone_count: u32,
}

#[derive(Clone)]
pub struct TraitorState {
    /// Remaining traitors for each team
    team_states: GroupVec<TeamState>,
    rng_state: Lcg64Xsh32,
}

impl TraitorState {
    pub fn new(team_count: usize, stone_count: u32, seed: u64, rule: &TraitorGo) -> Self {
        Self {
            team_states: vec![
                TeamState {
                    stone_count,
                    traitor_count: rule.traitor_count,
                };
                team_count
            ]
            .as_slice()
            .into(),
            rng_state: Lcg64Xsh32::seed_from_u64(seed),
        }
    }

    pub fn next_color(&mut self, team_color: Color, rule: &TraitorGo) -> Color {
        let team = &mut self.team_states[team_color.as_usize() - 1];
        let stone_count = team.stone_count;
        let _ = team.stone_count.saturating_sub(1);

        if team.traitor_count > 0 && self.rng_state.next_u32() % (rule.traitor_count * stone_count/4) == 0 {
            team.traitor_count -= 1;

            let mut color = self.rng_state.next_u32() as usize % (self.team_states.len() - 1) + 1;
            if color == team_color.as_usize() {
                color += 1;
            }
            Color(color as u8)
        } else {
            team_color
        }
    }
}
