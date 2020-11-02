use crate::game::{Board, GroupVec, NPlusOne, Point, Visibility, VisibilityBoard};

pub enum NPlusOneResult {
    ExtraTurn,
    Nothing,
}

pub fn check(
    points_played: &GroupVec<Point>,
    board: &Board,
    mut visibility: Option<&mut VisibilityBoard>,
    rule: &NPlusOne,
) -> NPlusOneResult {
    let mut line_points = Vec::new();

    let mut matched = false;

    for &point_played in points_played {
        let color = board.get_point(point_played);

        if color.is_empty() {
            // The point can be empty if the group was killed by traitor-suicide.
            // Skip it in that case.
            continue;
        }

        let add_point = |line_points: &mut Vec<Point>, p: Point| {
            if board.get_point(p) == color && !line_points.contains(&p) {
                line_points.push(p);
                false
            } else {
                true
            }
        };

        // Vertical ///////////////////////////////////////////////////////////

        let mut y = point_played.1 as i32 - 1;
        while let Some(p) = board.wrap_point(point_played.0 as i32, y) {
            if add_point(&mut line_points, p) {
                break;
            }
            y -= 1;
        }

        let mut y = point_played.1 as i32;
        while let Some(p) = board.wrap_point(point_played.0 as i32, y) {
            if add_point(&mut line_points, p) {
                break;
            }
            y += 1;
        }

        let vertical_match = line_points.len() == rule.length as usize;

        if vertical_match {
            if let Some(visibility) = visibility.as_mut() {
                for &p in &line_points {
                    *visibility.point_mut(p) = Visibility::new();
                }
            }
        }

        line_points.clear();

        // Horizontal /////////////////////////////////////////////////////////

        let mut x = point_played.0 as i32 - 1;
        while let Some(p) = board.wrap_point(x, point_played.1 as i32) {
            if add_point(&mut line_points, p) {
                break;
            }
            x -= 1;
        }

        let mut x = point_played.0 as i32;
        while let Some(p) = board.wrap_point(x, point_played.1 as i32) {
            if add_point(&mut line_points, p) {
                break;
            }
            x += 1;
        }

        let horizontal_match = line_points.len() == rule.length as usize;

        if horizontal_match {
            if let Some(visibility) = visibility.as_mut() {
                for &p in &line_points {
                    *visibility.point_mut(p) = Visibility::new();
                }
            }
        }

        line_points.clear();

        // Diagonal top left - bottom right ///////////////////////////////////

        let mut point = (point_played.0 as i32 - 1, point_played.1 as i32 - 1);
        while let Some(p) = board.wrap_point(point.0, point.1) {
            if add_point(&mut line_points, p) {
                break;
            }

            point.0 -= 1;
            point.1 -= 1;
        }

        let mut point = (point_played.0 as i32, point_played.1 as i32);
        while let Some(p) = board.wrap_point(point.0, point.1) {
            if add_point(&mut line_points, p) {
                break;
            }

            point.0 += 1;
            point.1 += 1;
        }

        let diagonal_tlbr_match = line_points.len() == rule.length as usize;

        if diagonal_tlbr_match {
            if let Some(visibility) = visibility.as_mut() {
                for &p in &line_points {
                    *visibility.point_mut(p) = Visibility::new();
                }
            }
        }

        line_points.clear();

        // Diagonal bottom left - top right ///////////////////////////////////

        let mut point = (point_played.0 as i32 - 1, point_played.1 as i32 + 1);
        while let Some(p) = board.wrap_point(point.0, point.1) {
            if add_point(&mut line_points, p) {
                break;
            }

            point.0 -= 1;
            point.1 += 1;
        }

        let mut point = (point_played.0 as i32, point_played.1 as i32);
        while let Some(p) = board.wrap_point(point.0, point.1) {
            if add_point(&mut line_points, p) {
                break;
            }

            point.0 += 1;
            point.1 -= 1;
        }

        let diagonal_bltr_match = line_points.len() == rule.length as usize;

        if diagonal_bltr_match {
            if let Some(visibility) = visibility.as_mut() {
                for &p in &line_points {
                    *visibility.point_mut(p) = Visibility::new();
                }
            }
        }

        line_points.clear();

        matched = matched
            || vertical_match
            || horizontal_match
            || diagonal_tlbr_match
            || diagonal_bltr_match;
    }

    if matched {
        return NPlusOneResult::ExtraTurn;
    }

    NPlusOneResult::Nothing
}
