use shared::game::{GameStateView, Visibility};
use web_sys::wasm_bindgen::JsCast;
use web_sys::DomRect;
use web_sys::{wasm_bindgen::JsValue, HtmlCanvasElement};

use crate::palette::Palette;
use crate::state::{self, GameHistory};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Input {
    Place((u32, u32), bool),
    Move(Direction, bool),
    WiderEdge,
    SmallerEdge,
    None,
}

pub(crate) struct Board {
    pub(crate) palette: Palette,
    pub(crate) toroidal_edge_size: i32,
    pub(crate) board_displacement: (i32, i32),
    pub(crate) selection_pos: Option<(u32, u32)>,
    pub(crate) input: Input,
    pub(crate) show_hidden: bool,
    pub(crate) edge_size: f64,
}

impl Input {
    pub(crate) fn from_pointer(
        board: &Board,
        game: &state::GameView,
        mut p: (f64, f64),
        bounding: DomRect,
        clicked: bool,
    ) -> Input {
        let pixel_ratio = gloo_utils::window().device_pixel_ratio();
        let edge_size = board.edge_size as f64 / pixel_ratio;
        let width = bounding.width() - (2.0 * edge_size);
        let height = bounding.height() - (2.0 * edge_size);
        let is_scoring = matches!(game.state, GameStateView::Scoring(_));

        // Adjust the coordinates for canvas position
        p.0 -= bounding.left();
        p.1 -= bounding.top();

        if p.0 < edge_size {
            return Input::Move(Direction::Left, clicked);
        }
        if p.1 < edge_size {
            return Input::Move(Direction::Up, clicked);
        }
        if p.0 > width + edge_size {
            return Input::Move(Direction::Right, clicked);
        }
        if p.1 > height + edge_size {
            return Input::Move(Direction::Down, clicked);
        }

        p.0 -= edge_size;
        p.1 -= edge_size;
        let size = (game.size.0 as i32 + 2 * board.toroidal_edge_size) as f64;
        let pos = match game.mods.pixel && !is_scoring {
            true => (
                (p.0 / (width / size) + 0.5) as i32,
                (p.1 / (height / size) + 0.5) as i32,
            ),
            false => (
                (p.0 / (width / size)) as i32,
                (p.1 / (height / size)) as i32,
            ),
        };

        Input::Place((pos.0 as u32, pos.1 as u32), clicked)
    }

    pub(crate) fn into_selection(self) -> Option<(u32, u32)> {
        match self {
            Input::Place(p, _) => Some(p),
            _ => None,
        }
    }
}

impl Board {
    pub(crate) fn render_gl(
        &self,
        canvas: &HtmlCanvasElement,
        game: &state::GameView,
        history: Option<&GameHistory>,
    ) -> Result<(), JsValue> {
        // Stone colors ///////////////////////////////////////////////////////

        let palette = &self.palette;
        let shadow_stone_colors = palette.shadow_stone_colors;
        let shadow_border_colors = palette.shadow_border_colors;
        let stone_colors = palette.stone_colors;
        let stone_colors_hidden = palette.stone_colors_hidden;
        let border_colors = palette.border_colors;
        let dead_mark_color = palette.dead_mark_color;

        let edge_size = self.edge_size;

        // Setup //////////////////////////////////////////////////////////////

        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        // let dpi = gloo_utils::window().device_pixel_ratio();
        // context.scale(dpi, dpi)?;

        let board = match history {
            Some(h) => &h.board,
            None => &game.board,
        };
        let board_visibility = match history {
            Some(h) => &h.board_visibility,
            None => &game.board_visibility,
        };

        // TODO: actually handle non-square boards
        let view_board_size = game.size.0 as usize + 2 * self.toroidal_edge_size as usize;
        let board_size = game.size.0 as usize;
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
        let size = (canvas.width() as f64 - 2.0 * edge_size) / view_board_size as f64;
        let turn = game.seats[game.turn as usize].team.0;

        let draw_stone =
            |(x, y): (i32, i32), diameter: f64, fill: bool, stroke: bool| -> Result<(), JsValue> {
                context.begin_path();
                context.arc(
                    edge_size + (x as f64 + 0.5) * size,
                    edge_size + (y as f64 + 0.5) * size,
                    diameter / 2.,
                    0.0,
                    2.0 * std::f64::consts::PI,
                )?;
                if fill {
                    context.fill();
                }
                if stroke {
                    context.stroke();
                }
                Ok(())
            };

        // Clear canvas ///////////////////////////////////////////////////////

        context.clear_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());

        context.set_fill_style(&JsValue::from_str(palette.background));
        context.fill_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());

        // Toroidal edge scroll boxes /////////////////////////////////////////

        if game.mods.toroidal.is_some() {
            context.set_stroke_style(&JsValue::from_str("#000000"));
            context.set_fill_style(&JsValue::from_str("#000000aa"));
            match self.input {
                Input::Move(Direction::Left, _) => {
                    context.fill_rect(0.0, 0.0, edge_size, height);
                }
                Input::Move(Direction::Right, _) => {
                    context.fill_rect(width - edge_size, 0.0, edge_size, height);
                }
                Input::Move(Direction::Up, _) => {
                    context.fill_rect(0.0, 0.0, width, edge_size);
                }
                Input::Move(Direction::Down, _) => {
                    context.fill_rect(0.0, height - edge_size, width, edge_size);
                }
                _ => {}
            }
        }

        // Board lines ////////////////////////////////////////////////////////

        context.set_line_width(1.0);
        context.set_stroke_style(&JsValue::from_str("#000000"));
        context.set_fill_style(&JsValue::from_str("#000000"));

        let line_edge_size = if game.mods.toroidal.is_some() {
            size / 2.0
        } else {
            0.0
        };

        for y in 0..view_board_size {
            context.begin_path();
            context.move_to(
                edge_size - line_edge_size + size * 0.5,
                edge_size + (y as f64 + 0.5) * size,
            );
            context.line_to(
                edge_size + line_edge_size + size * (view_board_size as f64 - 0.5),
                edge_size + (y as f64 + 0.5) * size,
            );
            context.stroke();
        }

        for x in 0..view_board_size {
            context.begin_path();
            context.move_to(
                edge_size + (x as f64 + 0.5) * size,
                edge_size - line_edge_size + size * 0.5,
            );
            context.line_to(
                edge_size + (x as f64 + 0.5) * size,
                edge_size + line_edge_size + size * (view_board_size as f64 - 0.5),
            );
            context.stroke();
        }

        // Starpoints /////////////////////////////////////////////////////////

        if game.mods.toroidal.is_none() {
            let points: &[(i32, i32)] = match game.size.0 {
                19 => &[
                    (3, 3),
                    (9, 3),
                    (15, 3),
                    (3, 9),
                    (9, 9),
                    (15, 9),
                    (3, 15),
                    (9, 15),
                    (15, 15),
                ],
                17 => &[
                    (3, 3),
                    (8, 3),
                    (13, 3),
                    (3, 8),
                    (8, 8),
                    (13, 8),
                    (3, 13),
                    (8, 13),
                    (13, 13),
                ],
                13 => &[(3, 3), (9, 3), (6, 6), (3, 9), (9, 9)],
                9 => &[(4, 4)],
                _ => &[],
            };
            for &(x, y) in points {
                let x = (x - self.board_displacement.0).rem_euclid(game.size.0 as i32);
                let y = (y - self.board_displacement.1).rem_euclid(game.size.1 as i32);
                draw_stone((x as _, y as _), size / 4., true, false)?;
            }
        }

        // Coordinates ////////////////////////////////////////////////////////

        let from_edge = edge_size - 20.0;

        context.set_font("bold 1.5em serif");

        context.set_text_align("center");
        context.set_text_baseline("middle");

        for (i, y) in (0..game.size.1)
            .cycle()
            .skip(
                self.board_displacement.1 as usize + board_size - self.toroidal_edge_size as usize,
            )
            .take(view_board_size)
            .enumerate()
        {
            let text = (game.size.1 - y).to_string();
            let i = i as f64 + 0.5;
            context.fill_text(&text, from_edge, edge_size + i * size + 2.0)?;
            context.fill_text(&text, width - from_edge, edge_size + i * size + 2.0)?;
        }

        context.set_text_align("center");
        context.set_text_baseline("baseline");

        for (i, x) in (0..game.size.0)
            .cycle()
            .skip(
                self.board_displacement.0 as usize + board_size - self.toroidal_edge_size as usize,
            )
            .take(view_board_size)
            .enumerate()
        {
            let letter = ('A'..'I')
                .chain('J'..='Z')
                .nth(x as usize)
                .unwrap()
                .to_string();
            let i = i as f64 + 0.5;
            context.fill_text(&letter, edge_size + i * size, from_edge)?;
            context.fill_text(&letter, edge_size + i * size, height - from_edge)?;
        }

        // Mouse hover display ////////////////////////////////////////////////

        let is_scoring = matches!(game.state, GameStateView::Scoring(_));
        if !is_scoring {
            if let Some(selection_pos) = self.selection_pos {
                let mut p = self.view_to_board_coord(game, selection_pos);
                if game.mods.pixel {
                    p.0 -= 1;
                    p.1 -= 1;
                }

                // TODO: This allocation is horrible, figure out how to avoid it
                // TODO: Also move these to shared
                let points = match game.mods.pixel {
                    true => vec![
                        (p.0, p.1),
                        (p.0 + 1, p.1),
                        (p.0, p.1 + 1),
                        (p.0 + 1, p.1 + 1),
                    ],
                    false => vec![p],
                };
                let color = turn;
                // Teams start from 1
                context.set_fill_style(&JsValue::from_str(shadow_stone_colors[color as usize - 1]));
                context
                    .set_stroke_style(&JsValue::from_str(shadow_border_colors[color as usize - 1]));

                for p in points {
                    self.board_to_view_coord(game, p, |p| {
                        draw_stone(p, size, true, true).unwrap();
                    });
                }
            }
        }

        // Board stones ///////////////////////////////////////////////////////

        for (idx, &color) in board.iter().enumerate() {
            let x = idx % board_size;
            let y = idx / board_size;

            let visible = board_visibility
                .as_ref()
                .map(|v| v[idx] == 0)
                .unwrap_or(true);

            if color.0 == 0 || !visible {
                continue;
            }

            context.set_fill_style(&JsValue::from_str(stone_colors[color.0 as usize - 1]));
            context.set_stroke_style(&JsValue::from_str(border_colors[color.0 as usize - 1]));

            self.board_to_view_coord(game, (x as i32, y as i32), |(px, py)| {
                draw_stone((px as _, py as _), size, true, true).unwrap();
            });
        }

        // Hidden stones //////////////////////////////////////////////////////

        if self.show_hidden {
            for (idx, &colors) in board_visibility.iter().flatten().enumerate() {
                let x = idx % board_size;
                let y = idx / board_size;

                let colors = Visibility::from_value(colors);

                if colors.is_empty() {
                    continue;
                }

                for color in &colors {
                    context.set_fill_style(&JsValue::from_str(
                        stone_colors_hidden[color as usize - 1],
                    ));
                    context.set_stroke_style(&JsValue::from_str(border_colors[color as usize - 1]));

                    self.board_to_view_coord(game, (x as i32, y as i32), |(px, py)| {
                        draw_stone((px as _, py as _), size, true, true).unwrap();
                    });
                }
            }
        }

        // Last stone marker //////////////////////////////////////////////////

        let last_stone = match (&game.state, history) {
            (_, Some(h)) => h.last_stone.as_ref(),
            (GameStateView::Play(state), _) => state.last_stone.as_ref(),
            _ => None,
        };

        if let Some(points) = last_stone {
            for &(x, y) in points {
                let mut color = board[y as usize * game.size.0 as usize + x as usize].0;

                if color == 0 {
                    // White stones have the most fitting (read: black) marker for empty board
                    color = 2;
                }

                context.set_stroke_style(&JsValue::from_str(dead_mark_color[color as usize - 1]));
                context.set_line_width(2.0);

                self.board_to_view_coord(game, (x as i32, y as i32), |(px, py)| {
                    draw_stone((px as _, py as _), size / 2., false, true).unwrap();
                });
            }
        }

        // States /////////////////////////////////////////////////////////////

        if history.is_none() {
            match &game.state {
                GameStateView::Scoring(scoring) | GameStateView::Done(scoring) => {
                    for group in &scoring.groups {
                        if group.alive {
                            continue;
                        }

                        for &(x, y) in &group.points {
                            self.board_to_view_coord(game, (x as i32, y as i32), |(x, y)| {
                                context.set_line_width(2.0);
                                context.set_stroke_style(&JsValue::from_str(
                                    dead_mark_color[group.team.0 as usize - 1],
                                ));

                                context.set_stroke_style(&JsValue::from_str(
                                    dead_mark_color[group.team.0 as usize - 1],
                                ));

                                context.begin_path();
                                context.move_to(
                                    edge_size + (x as f64 + 0.2) * size,
                                    edge_size + (y as f64 + 0.2) * size,
                                );
                                context.line_to(
                                    edge_size + (x as f64 + 0.8) * size,
                                    edge_size + (y as f64 + 0.8) * size,
                                );
                                context.stroke();

                                context.begin_path();
                                context.move_to(
                                    edge_size + (x as f64 + 0.8) * size,
                                    edge_size + (y as f64 + 0.2) * size,
                                );
                                context.line_to(
                                    edge_size + (x as f64 + 0.2) * size,
                                    edge_size + (y as f64 + 0.8) * size,
                                );
                                context.stroke();
                            });
                        }
                    }

                    for (idx, &color) in scoring.points.points.iter().enumerate() {
                        let x = idx % board_size;
                        let y = idx / board_size;

                        if color.is_empty() {
                            continue;
                        }

                        self.board_to_view_coord(game, (x as i32, y as i32), |(x, y)| {
                            context.set_fill_style(&JsValue::from_str(
                                stone_colors[color.0 as usize - 1],
                            ));

                            context.set_stroke_style(&JsValue::from_str(
                                border_colors[color.0 as usize - 1],
                            ));

                            context.fill_rect(
                                edge_size + (x as f64 + 1. / 3.) * size,
                                edge_size + (y as f64 + 1. / 3.) * size,
                                (1. / 3.) * size,
                                (1. / 3.) * size,
                            );
                        });
                    }
                }
                _ => {}
            }
        }

        // Toroidal edge grayout //////////////////////////////////////////////

        if game.mods.toroidal.is_some() {
            context.set_stroke_style(&JsValue::from_str("#000000"));
            context.set_fill_style(&JsValue::from_str("#00000055"));
            let e = self.toroidal_edge_size as f64;
            context.fill_rect(edge_size, edge_size, e * size, height - edge_size * 2.0);
            context.fill_rect(
                edge_size + e * size,
                edge_size,
                width - edge_size * 2.0 - 2.0 * e * size,
                e * size,
            );
            context.fill_rect(
                width - e * size - edge_size,
                edge_size,
                e * size,
                height - edge_size * 2.0,
            );
            context.fill_rect(
                edge_size + e * size,
                height - e * size - edge_size,
                width - edge_size * 2.0 - 2.0 * e * size,
                e * size,
            );
        }

        Ok(())
    }

    fn view_to_board_coord(&self, game: &state::GameView, view: (u32, u32)) -> (i32, i32) {
        let edge = self.toroidal_edge_size;
        let size = game.size.0 as i32;
        let mut x = view.0 as i32;
        let mut y = view.1 as i32;

        if game.mods.toroidal.is_none() {
            return (x, y);
        }

        x -= edge;
        y -= edge;

        if x < 0 {
            x += size;
        }
        if y < 1 {
            y += size;
        }
        if x >= size {
            x -= size;
        }
        if y >= size {
            y -= size;
        }

        x = (x + self.board_displacement.0).rem_euclid(game.size.0 as i32);
        y = (y + self.board_displacement.1).rem_euclid(game.size.1 as i32);

        (x, y)
    }

    fn board_to_view_coord(
        &self,
        game: &state::GameView,
        board: (i32, i32),
        mut cb: impl FnMut((i32, i32)),
    ) {
        let edge = self.toroidal_edge_size;
        let size = game.size.0 as i32;

        if game.mods.toroidal.is_none()
            && (board.0 < 0 || board.1 < 0 || board.0 >= size || board.1 >= size)
        {
            return;
        }

        let x = (board.0 - self.board_displacement.0).rem_euclid(game.size.0 as i32);
        let y = (board.1 - self.board_displacement.1).rem_euclid(game.size.1 as i32);
        cb((x + edge, y + edge));

        if x < edge {
            cb((x + size + edge, y + edge));
            if y < edge {
                cb((x + size + edge, y + size + edge));
            }
            if y >= size - edge {
                cb((x + size + edge, y - size + edge));
            }
        }
        if y < edge {
            cb((x + edge, y + size + edge));
        }
        if x >= size - edge {
            cb((x - size + edge, y + edge));
            if y < edge {
                cb((x - size + edge, y + size + edge));
            }
            if y >= size - edge {
                cb((x - size + edge, y - size + edge));
            }
        }
        if y >= size - edge {
            cb((x + edge, y - size + edge))
        }
    }
}
