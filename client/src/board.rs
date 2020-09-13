use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d as Canvas2d;
use web_sys::HtmlCanvasElement;
use yew::services::{RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

use shared::game::{GameState, Visibility};
use shared::message::{ClientMessage, GameAction};

use crate::game_view::GameView;
use crate::networking;

pub struct Board {
    props: Props,
    canvas: Option<HtmlCanvasElement>,
    canvas2d: Option<Canvas2d>,
    link: ComponentLink<Self>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    mouse_pos: Option<(f64, f64)>,
    selection_pos: Option<(u32, u32)>,
    width: u32,
    height: u32,
    edge_size: i32,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub game: GameView,
    pub size: i32,
}

pub enum Msg {
    Render(f64),
    MouseMove((f64, f64)),
    Click((f64, f64)),
    MouseLeave,
}

impl Component for Board {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        Board {
            props,
            canvas: None,
            canvas2d: None,
            link,
            node_ref: NodeRef::default(),
            render_loop: None,
            mouse_pos: None,
            selection_pos: None,
            width: 0,
            height: 0,
            edge_size: 30,
        }
    }

    fn rendered(&mut self, first_render: bool) {
        // Once rendered, store references for the canvas and GL context. These can be used for
        // resizing the rendering area when the window or canvas element are resized, as well as
        // for making GL calls.

        let canvas = self.node_ref.cast::<HtmlCanvasElement>().unwrap();

        let canvas2d: Canvas2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into()
            .unwrap();

        {
            let mouse_move = self.link.callback(Msg::MouseMove);
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_move.emit((event.offset_x() as f64, event.offset_y() as f64));
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        {
            let mouse_click = self.link.callback(Msg::Click);
            let closure = Closure::wrap(Box::new(move |event: web_sys::MouseEvent| {
                mouse_click.emit((event.offset_x() as f64, event.offset_y() as f64));
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mousedown", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        {
            let mouse_leave = self.link.callback(|_| Msg::MouseLeave);
            let closure = Closure::wrap(Box::new(move |_event: web_sys::MouseEvent| {
                mouse_leave.emit(());
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mouseleave", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        self.width = canvas.width();
        self.height = canvas.height();

        self.canvas = Some(canvas);
        self.canvas2d = Some(canvas2d);

        // In a more complex use-case, there will be additional WebGL initialization that should be
        // done here, such as enabling or disabling depth testing, depth functions, face
        // culling etc.

        if first_render {
            self.render_gl(0.0).unwrap();
            // The callback to request animation frame is passed a time value which can be used for
            // rendering motion independent of the framerate which may vary.
            let render_frame = self.link.callback(Msg::Render);
            let handle = RenderService::request_animation_frame(render_frame);

            // A reference to the handle must be stored, otherwise it is dropped and the render won't
            // occur.
            self.render_loop = Some(Box::new(handle));
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            if let Some(canvas) = &self.canvas {
                let window = web_sys::window().unwrap();
                let pixel_ratio = window.device_pixel_ratio();

                let size = self.props.size;
                let scaled_size = size as f64 * pixel_ratio;

                canvas.set_width(scaled_size as u32);
                canvas.set_height(scaled_size as u32);
                let _ = canvas.style().set_property("width", &format!("{}px", size));
                let _ = canvas
                    .style()
                    .set_property("height", &format!("{}px", size));

                self.width = size as u32;
                self.height = size as u32;
            }
            self.render_gl(0.0).unwrap();
            false
        } else {
            false
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        let game = &self.props.game;
        let edge_size = self.edge_size as f64;
        let width = self.width as f64 - (edge_size);
        let height = self.height as f64 - (edge_size);
        let mouse_to_coord = |mut p: (f64, f64)| -> Option<(u32, u32)> {
            if p.0 < edge_size || p.1 < edge_size || p.0 > width || p.1 > height {
                return None;
            }

            p.0 -= edge_size;
            p.1 -= edge_size;
            Some(match game.mods.pixel {
                true => (
                    (p.0 / (width / game.size.0 as f64) + 0.5) as u32,
                    (p.1 / (width / game.size.1 as f64) + 0.5) as u32,
                ),
                false => (
                    (p.0 / (width / game.size.0 as f64)) as u32,
                    (p.1 / (width / game.size.1 as f64)) as u32,
                ),
            })
        };

        match msg {
            Msg::Render(_timestamp) => {
                //self.render_gl(timestamp).unwrap();
            }
            Msg::MouseMove(p) => {
                self.mouse_pos = Some(p);
                self.selection_pos = mouse_to_coord(p);
                self.render_gl(0.0).unwrap();
            }
            Msg::Click(p) => {
                // Ignore clicks while viewing history
                if self.props.game.history.is_some() {
                    return false;
                }
                self.mouse_pos = Some(p);
                self.selection_pos = mouse_to_coord(p);
                if let Some(selection_pos) = self.selection_pos {
                    networking::send(ClientMessage::GameAction(GameAction::Place(
                        selection_pos.0,
                        selection_pos.1,
                    )));
                }
            }
            Msg::MouseLeave => {
                self.mouse_pos = None;
                self.selection_pos = None;
                self.render_gl(0.0).unwrap();
            }
        }
        false
    }

    fn view(&self) -> Html {
        html! {
            <canvas ref={self.node_ref.clone()} width=self.props.size height=self.props.size />
        }
    }
}

impl Board {
    fn render_gl(&mut self, _timestamp: f64) -> Result<(), JsValue> {
        // Stone colors ///////////////////////////////////////////////////////

        let shadow_stone_colors = ["#555555", "#bbbbbb", "#7b91bd", "#e09db4"];
        let shadow_border_colors = ["#bbbbbb", "#555555", "#555555", "#555555"];
        let stone_colors = ["#000000", "#eeeeee", "#5074bc", "#e0658f"];
        let stone_colors_hidden = ["#00000080", "#eeeeee80", "#5074bc80", "#e0658f80"];
        let border_colors = ["#555555", "#000000", "#000000", "#000000"];
        let dead_mark_color = ["#eeeeee", "#000000", "#000000", "#000000"];

        let edge_size = self.edge_size as f64;

        // Setup //////////////////////////////////////////////////////////////

        let context = self
            .canvas2d
            .as_ref()
            .expect("Canvas Context not initialized!");
        let canvas = self.canvas.as_ref().expect("Canvas not initialized!");

        let game = &self.props.game;
        let board = match &game.history {
            Some(h) => &h.board,
            None => &game.board,
        };
        let board_visibility = match &game.history {
            Some(h) => &h.board_visibility,
            None => &game.board_visibility,
        };

        // TODO: actually handle non-square boards
        let board_size = game.size.0 as usize;
        let size = (canvas.width() as f64 - 2.0 * edge_size) / board_size as f64;
        let turn = game.seats[game.turn as usize].1;

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

        context.set_fill_style(&JsValue::from_str("#e0bb6c"));
        context.fill_rect(0.0, 0.0, canvas.width().into(), canvas.height().into());

        // Board lines ////////////////////////////////////////////////////////

        context.set_line_width(1.0);
        context.set_stroke_style(&JsValue::from_str("#000000"));
        context.set_fill_style(&JsValue::from_str("#000000"));

        for y in 0..game.size.1 {
            context.begin_path();
            context.move_to(edge_size + size * 0.5, edge_size + (y as f64 + 0.5) * size);
            context.line_to(
                edge_size + size * (board_size as f64 - 0.5),
                edge_size + (y as f64 + 0.5) * size,
            );
            context.stroke();
        }

        for x in 0..game.size.0 {
            context.begin_path();
            context.move_to(edge_size + (x as f64 + 0.5) * size, edge_size + size * 0.5);
            context.line_to(
                edge_size + (x as f64 + 0.5) * size,
                edge_size + size * (board_size as f64 - 0.5),
            );
            context.stroke();
        }

        // Starpoints /////////////////////////////////////////////////////////

        let points: &[(u8, u8)] = match game.size.0 {
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
            13 => &[(3, 3), (9, 3), (6, 6), (3, 9), (9, 9)],
            9 => &[(4, 4)],
            _ => &[],
        };
        for &(x, y) in points {
            draw_stone((x as _, y as _), size / 4., true, false)?;
        }

        // Coordinates ////////////////////////////////////////////////////////

        context.set_font("bold 20px serifd");

        context.set_text_align("center");
        context.set_text_baseline("middle");

        for y in 0..game.size.1 {
            let text = (game.size.1 - y).to_string();
            let y = y as f64 + 0.5;
            context.fill_text(&text, edge_size - 15.0, edge_size + y as f64 * size + 2.0)?;
            context.fill_text(
                &text,
                self.width as f64 - edge_size + 15.0,
                edge_size + y as f64 * size + 2.0,
            )?;
        }

        context.set_text_align("center");
        context.set_text_baseline("baseline");

        for x in 0..game.size.0 {
            let letter = ('A'..'I')
                .chain('J'..'Z')
                .nth(x as usize)
                .unwrap()
                .to_string();
            let x = x as f64 + 0.5;
            context.fill_text(&letter, edge_size + x as f64 * size, edge_size - 10.0)?;
            context.fill_text(
                &letter,
                edge_size + x as f64 * size,
                self.height as f64 - edge_size + 15.0,
            )?;
        }

        // Mouse hover display ////////////////////////////////////////////////

        if let Some(selection_pos) = self.selection_pos {
            let mut p = (selection_pos.0 as i32, selection_pos.1 as i32);
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
            context.set_stroke_style(&JsValue::from_str(shadow_border_colors[color as usize - 1]));

            for p in points {
                if p.0 < 0 || p.1 < 0 || p.0 >= game.size.0 as i32 || p.1 >= game.size.1 as i32 {
                    continue;
                }
                draw_stone(p, size, true, true)?;
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

            if color == 0 || !visible {
                continue;
            }

            context.set_fill_style(&JsValue::from_str(stone_colors[color as usize - 1]));
            context.set_stroke_style(&JsValue::from_str(border_colors[color as usize - 1]));

            draw_stone((x as _, y as _), size, true, true)?;
        }

        // Hidden stones //////////////////////////////////////////////////////

        for (idx, &colors) in board_visibility.iter().flatten().enumerate() {
            let x = idx % board_size;
            let y = idx / board_size;

            let colors = Visibility::from_value(colors);

            if colors.is_empty() {
                continue;
            }

            for color in &colors {
                context.set_fill_style(&JsValue::from_str(stone_colors_hidden[color as usize - 1]));
                context.set_stroke_style(&JsValue::from_str(border_colors[color as usize - 1]));

                draw_stone((x as _, y as _), size, true, true)?;
            }
        }

        // Last stone marker //////////////////////////////////////////////////

        let last_stone = match (&game.state, &game.history) {
            (_, Some(h)) => h.last_stone.as_ref(),
            (GameState::Play(state), _) => state.last_stone.as_ref(),
            _ => None,
        };

        if let Some(points) = last_stone {
            for &(x, y) in points {
                let mut color = board[y as usize * game.size.0 as usize + x as usize];

                if color == 0 {
                    // White stones have the most fitting (read: black) marker for empty board
                    color = 2;
                }

                context.set_stroke_style(&JsValue::from_str(dead_mark_color[color as usize - 1]));
                context.set_line_width(2.0);

                draw_stone((x as _, y as _), size / 2., false, true)?;
            }
        }

        // States /////////////////////////////////////////////////////////////

        match &game.state {
            GameState::Scoring(scoring) | GameState::Done(scoring) => {
                for group in &scoring.groups {
                    if group.alive {
                        continue;
                    }

                    for &(x, y) in &group.points {
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
                    }
                }

                for (idx, &color) in scoring.points.points.iter().enumerate() {
                    let x = (idx % board_size) as f64;
                    let y = (idx / board_size) as f64;

                    if color.is_empty() {
                        continue;
                    }

                    context.set_fill_style(&JsValue::from_str(stone_colors[color.0 as usize - 1]));

                    context
                        .set_stroke_style(&JsValue::from_str(border_colors[color.0 as usize - 1]));

                    context.fill_rect(
                        edge_size + (x + 1. / 3.) * size,
                        edge_size + (y + 1. / 3.) * size,
                        edge_size + (1. / 3.) * size,
                        edge_size + (1. / 3.) * size,
                    );
                }
            }
            _ => {}
        }

        let render_frame = self.link.callback(Msg::Render);
        let handle = RenderService::request_animation_frame(render_frame);

        // A reference to the new handle must be retained for the next render to run.
        self.render_loop = Some(Box::new(handle));

        Ok(())
    }
}
