use crate::game::find_groups;
use crate::game::Color;
use crate::game::{Board, GroupVec, Point, TetrisGo, VisibilityBoard};

use super::reveal_group;
use super::Revealed;

pub enum TetrisResult {
    Nothing,
    Illegal(Revealed),
}

pub fn check(
    points_played: &mut GroupVec<Point>,
    board: &mut Board,
    mut visibility: Option<&mut VisibilityBoard>,
    _rule: &TetrisGo,
) -> TetrisResult {
    let groups = find_groups(board);
    let mut revealed = false;

    for point_played in points_played.clone() {
        let color = board.get_point(point_played);

        for group in &groups {
            if group.team != color || group.points.len() != 4 {
                continue;
            }

            let contains = group.points.contains(&point_played);

            if !contains {
                continue;
            }

            points_played.retain(|x| *x != point_played);
            if let Some(visibility) = visibility.as_mut() {
                revealed = revealed || reveal_group(Some(visibility), group, board);
            }
            *board.point_mut(point_played) = Color::empty();
        }
    }

    if points_played.is_empty() {
        return TetrisResult::Illegal(revealed);
    }

    TetrisResult::Nothing
}
