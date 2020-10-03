use crate::game::find_groups;
use crate::game::Color;
use crate::game::{Board, GroupVec, Point, TetrisGo};

pub enum TetrisResult {
    Nothing,
    Illegal,
}

pub fn check(
    points_played: &mut GroupVec<Point>,
    board: &mut Board,
    _rule: &TetrisGo,
) -> TetrisResult {
    let groups = find_groups(board);

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
            *board.point_mut(point_played) = Color::empty();
        }
    }

    if points_played.is_empty() {
        return TetrisResult::Illegal;
    }

    TetrisResult::Nothing
}
