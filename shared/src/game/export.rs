use super::Board;
use super::Game;
use std::fmt::Write;

struct SGFWriter {
    buffer: String,
}

impl SGFWriter {
    fn new() -> SGFWriter {
        SGFWriter {
            buffer: "(;FF[4]GM[1]".to_string(),
        }
    }

    fn size(&mut self, size: (u32, u32)) {
        if size.0 == size.1 {
            let _ = write!(&mut self.buffer, "SZ[{}]", size.0);
        } else {
            let _ = write!(&mut self.buffer, "SZ[{}:{}]", size.0, size.1);
        }
    }

    fn set_point(&mut self, point: (u32, u32), color: u8) {
        let name = match color {
            0 => "AE",
            1 => "AB",
            2 => "AW",
            _ => unreachable!(),
        };

        let (x, y) = self.point(point);

        let _ = write!(&mut self.buffer, "{}[{}{}]", name, x, y);
    }

    fn point(&self, point: (u32, u32)) -> (char, char) {
        let mut letters = 'a'..='z';
        let x = letters.clone().nth(point.0 as usize).unwrap_or('a');
        let y = letters.nth(point.1 as usize).unwrap_or('a');
        (x, y)
    }

    fn label(&mut self, point: (u32, u32), text: &str) {
        let (x, y) = self.point(point);

        let _ = write!(&mut self.buffer, "LB[{}{}:{}]", x, y, text);
    }

    fn end_turn(&mut self) {
        let _ = write!(&mut self.buffer, ";");
    }

    fn finish(mut self) -> String {
        let _ = write!(&mut self.buffer, ")");
        self.buffer
    }
}

/// Write a simple single-variation representation of the game.
/// Limited to two colors so has to use markers for the other colors and hidden stones.
pub fn sgf_export(game: &Game) -> String {
    let mut writer = SGFWriter::new();
    let (width, height) = (game.shared.board.width, game.shared.board.height);
    writer.size((width, height));

    let mut last = Board::empty(width, height, game.shared.board.toroidal);

    for history in &game.shared.board_history {
        let board = &history.board;

        for (idx, (old, new)) in last.points.iter_mut().zip(&board.points).enumerate() {
            if *old != *new {
                // Map colored stones to black and white.
                let color = if new.0 == 0 { 0 } else { (new.0 - 1) % 2 + 1 };
                writer.set_point(board.idx_to_coord(idx).unwrap(), color);
                *old = *new;
            }
        }

        for (idx, new) in board.points.iter().enumerate() {
            let coord = board.idx_to_coord(idx).unwrap();
            match new.0 {
                3 => writer.label(coord, "U"),
                4 => writer.label(coord, "R"),
                _ => {}
            }
        }

        writer.end_turn();

        // TODO: PUZZLE markers for hidden stones
    }

    writer.finish()
}
