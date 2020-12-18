use js_sys::Date;
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::CanvasRenderingContext2d as Canvas2d;
use web_sys::HtmlCanvasElement;
use yew::services::{RenderService, Task};
use yew::{html, Component, ComponentLink, Html, NodeRef, Properties, ShouldRender};

use shared::game::{GameStateView, Visibility};
use shared::message::{ClientMessage, GameAction};

use crate::game_view::GameView;
use crate::networking;
use crate::palette::Palette;

// TODO: PUZZLE Move audio handling to its own agent.

struct Audio {
    stone_sounds: Vec<web_sys::HtmlAudioElement>,
    /// Number of milliseconds since epoch
    last_play: f64,
}

impl Audio {
    fn new() -> Audio {
        let stones = [
            "/sounds/stone1.wav",
            "/sounds/stone2.wav",
            "/sounds/stone3.wav",
            "/sounds/stone4.wav",
            "/sounds/stone5.wav",
        ];

        let stone_sounds = stones
            .iter()
            .filter_map(|path| web_sys::HtmlAudioElement::new_with_src(*path).ok())
            .collect();

        Audio {
            stone_sounds,
            last_play: Date::now(),
        }
    }

    fn play_stone(&mut self) {
        let time = Date::now();
        if time - self.last_play < 100.0 {
            return;
        }
        self.last_play = time;

        // Good enough randomness
        let idx = (time % self.stone_sounds.len() as f64) as usize;

        // We don't really care about playback errors.
        let sound = &self.stone_sounds[idx];
        sound.set_current_time(0.0);
        // TODO: PUZZLE unhardcode this
        sound.set_volume(0.25);
        let _ = sound.play();
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Direction {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Input {
    Place((u32, u32), bool),
    Move(Direction, bool),
    None,
}

impl Input {
    fn from_pointer(board: &Board, mut p: (f64, f64), clicked: bool) -> Input {
        let game = &board.props.game;
        let edge_size = board.edge_size as f64 / board.pixel_ratio;
        let width = board.width as f64 - (2.0 * edge_size);
        let height = board.height as f64 - (2.0 * edge_size);
        let is_scoring = matches!(game.state, GameStateView::Scoring(_));
        let bounding = board.canvas.as_ref().unwrap().get_bounding_client_rect();

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
        let pos = match game.mods.pixel && !is_scoring {
            true => (
                (p.0 / (width / game.size.0 as f64) + 0.5) as u32,
                (p.1 / (height / game.size.1 as f64) + 0.5) as u32,
            ),
            false => (
                (p.0 / (width / game.size.0 as f64)) as u32,
                (p.1 / (height / game.size.1 as f64)) as u32,
            ),
        };

        Input::Place(pos, clicked)
    }

    fn into_selection(self) -> Option<(u32, u32)> {
        match self {
            Input::Place(p, _) => Some(p),
            _ => None,
        }
    }
}

pub(crate) struct Board {
    props: Props,
    canvas: Option<HtmlCanvasElement>,
    canvas2d: Option<Canvas2d>,
    link: ComponentLink<Self>,
    node_ref: NodeRef,
    render_loop: Option<Box<dyn Task>>,
    selection_pos: Option<(u32, u32)>,
    width: u32,
    height: u32,
    edge_size: i32,
    pixel_ratio: f64,
    audio: Audio,
    input: Input,
    board_displacement: (i32, i32),
}

#[derive(Properties, Clone, PartialEq)]
pub(crate) struct Props {
    pub game: GameView,
    pub size: i32,
    pub show_hidden: bool,
    pub palette: Palette,
}

pub enum Msg {
    Render(f64),
    MouseMove((f64, f64)),
    Click((f64, f64, bool)),
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
            selection_pos: None,
            width: 0,
            height: 0,
            edge_size: 40,
            pixel_ratio: 1.0,
            audio: Audio::new(),
            input: Input::None,
            board_displacement: (0, 0),
        }
    }

    fn rendered(&mut self, first_render: bool) {
        // Once rendered, store references for the canvas and GL context.

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
                mouse_move.emit((event.client_x() as f64, event.client_y() as f64));
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("mousemove", closure.as_ref().unchecked_ref())
                .unwrap();
            closure.forget();
        }

        // TODO: Add proper touch event handlers.

        {
            let mouse_click = self.link.callback(Msg::Click);
            let closure = Closure::wrap(Box::new(move |event: web_sys::PointerEvent| {
                // Only trigger for primary button.
                // See https://developer.mozilla.org/en-US/docs/Web/API/Pointer_events
                println!("{:?}", event);
                let buttons = event.buttons();
                if event.is_primary() && (buttons == 0 || buttons == 1) {
                    let is_touch = event.pointer_type() == "touch";
                    mouse_click.emit((event.client_x() as f64, event.client_y() as f64, is_touch));
                }
            }) as Box<dyn FnMut(_)>);
            canvas
                .add_event_listener_with_callback("pointerdown", closure.as_ref().unchecked_ref())
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
            if self.props.game.move_number != props.game.move_number
                || self.props.game.history.as_ref().map(|x| x.move_number)
                    != props.game.history.as_ref().map(|x| x.move_number)
            {
                self.audio.play_stone();
            }

            if self.props.game.room_id != props.game.room_id {
                self.board_displacement = (0, 0);
            }

            self.props = props;
            if let Some(canvas) = &self.canvas {
                let window = web_sys::window().unwrap();
                let pixel_ratio = window.device_pixel_ratio();
                self.pixel_ratio = pixel_ratio;

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
        match msg {
            Msg::Render(_timestamp) => {
                //self.render_gl(timestamp).unwrap();
            }
            Msg::MouseMove(p) => {
                let input = Input::from_pointer(self, p, false);
                self.input = input;
                self.selection_pos = input.into_selection();
                self.render_gl(0.0).unwrap();
            }
            Msg::Click((x, y, is_touch)) => {
                let p = (x, y);
                let input = Input::from_pointer(self, p, false);
                let coord = input.into_selection();
                let send = !is_touch || self.selection_pos == coord;
                self.input = input;
                self.selection_pos = coord;
                if let Some(selection_pos) = self.selection_pos {
                    // Ignore clicks while viewing history
                    if self.props.game.history.is_some() {
                        return false;
                    }
                    if send {
                        let game = &self.props.game;
                        let x = (selection_pos.0 as i32 + self.board_displacement.0)
                            .rem_euclid(game.size.0 as i32) as u32;
                        let y = (selection_pos.1 as i32 + self.board_displacement.1)
                            .rem_euclid(game.size.1 as i32) as u32;
                        networking::send(ClientMessage::GameAction {
                            room_id: None,
                            action: GameAction::Place(x, y),
                        });
                    }
                }
                if self.props.game.mods.toroidal.is_some() {
                    let render = match input {
                        Input::Move(Direction::Left, _) => {
                            self.board_displacement.0 -= 1;
                            if self.board_displacement.0 < 0 {
                                self.board_displacement.0 += self.props.game.size.0 as i32;
                            }
                            true
                        }
                        Input::Move(Direction::Right, _) => {
                            self.board_displacement.0 += 1;
                            if self.board_displacement.0 >= self.props.game.size.0 as i32 {
                                self.board_displacement.0 = 0;
                            }
                            true
                        }
                        Input::Move(Direction::Up, _) => {
                            self.board_displacement.1 -= 1;
                            if self.board_displacement.1 < 0 {
                                self.board_displacement.1 += self.props.game.size.1 as i32;
                            }
                            true
                        }
                        Input::Move(Direction::Down, _) => {
                            self.board_displacement.1 += 1;
                            if self.board_displacement.1 >= self.props.game.size.1 as i32 {
                                self.board_displacement.1 = 0;
                            }
                            true
                        }
                        _ => false,
                    };
                    if render {
                        self.render_gl(0.0).unwrap();
                    }
                }
            }
            Msg::MouseLeave => {
                self.input = Input::None;
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

        let palette = &self.props.palette;
        let shadow_stone_colors = palette.shadow_stone_colors;
        let shadow_border_colors = palette.shadow_border_colors;
        let stone_colors = palette.stone_colors;
        let stone_colors_hidden = palette.stone_colors_hidden;
        let border_colors = palette.border_colors;
        let dead_mark_color = palette.dead_mark_color;

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
        let width = canvas.width() as f64;
        let height = canvas.height() as f64;
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
            edge_size / 2.0
        } else {
            0.0
        };

        for y in 0..game.size.1 {
            context.begin_path();
            context.move_to(
                edge_size - line_edge_size + size * 0.5,
                edge_size + (y as f64 + 0.5) * size,
            );
            context.line_to(
                edge_size + line_edge_size + size * (board_size as f64 - 0.5),
                edge_size + (y as f64 + 0.5) * size,
            );
            context.stroke();
        }

        for x in 0..game.size.0 {
            context.begin_path();
            context.move_to(
                edge_size + (x as f64 + 0.5) * size,
                edge_size - line_edge_size + size * 0.5,
            );
            context.line_to(
                edge_size + (x as f64 + 0.5) * size,
                edge_size + line_edge_size + size * (board_size as f64 - 0.5),
            );
            context.stroke();
        }

        // Starpoints /////////////////////////////////////////////////////////

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

        // Coordinates ////////////////////////////////////////////////////////

        let from_edge = edge_size - 20.0;

        context.set_font("bold 1.5em serif");

        context.set_text_align("center");
        context.set_text_baseline("middle");

        for (i, y) in (0..game.size.1)
            .cycle()
            .skip(self.board_displacement.1 as usize)
            .take(game.size.1 as usize)
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
            .skip(self.board_displacement.0 as usize)
            .take(game.size.0 as usize)
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
                context
                    .set_stroke_style(&JsValue::from_str(shadow_border_colors[color as usize - 1]));

                for p in points {
                    if p.0 < 0 || p.1 < 0 || p.0 >= game.size.0 as i32 || p.1 >= game.size.1 as i32
                    {
                        continue;
                    }
                    draw_stone(p, size, true, true)?;
                }
            }
        }

        // Board stones ///////////////////////////////////////////////////////

        for (idx, _) in board.iter().enumerate() {
            let x = idx % board_size;
            let y = idx / board_size;

            let px = (x as i32 + self.board_displacement.0).rem_euclid(game.size.0 as i32);
            let py = (y as i32 + self.board_displacement.1).rem_euclid(game.size.1 as i32);
            let pidx = py as usize * board_size + px as usize;
            let color = board[pidx];

            let visible = board_visibility
                .as_ref()
                .map(|v| v[pidx] == 0)
                .unwrap_or(true);

            if color == 0 || !visible {
                continue;
            }

            context.set_fill_style(&JsValue::from_str(stone_colors[color as usize - 1]));
            context.set_stroke_style(&JsValue::from_str(border_colors[color as usize - 1]));

            draw_stone((x as _, y as _), size, true, true)?;
        }

        // Hidden stones //////////////////////////////////////////////////////

        if self.props.show_hidden {
            for (idx, _) in board_visibility.iter().flatten().enumerate() {
                let x = idx % board_size;
                let y = idx / board_size;

                let px = (x as i32 + self.board_displacement.0).rem_euclid(game.size.0 as i32);
                let py = (y as i32 + self.board_displacement.1).rem_euclid(game.size.1 as i32);
                let pidx = py as usize * board_size + px as usize;
                let colors = board_visibility.as_ref().unwrap()[pidx];

                let colors = Visibility::from_value(colors);

                if colors.is_empty() {
                    continue;
                }

                for color in &colors {
                    context.set_fill_style(&JsValue::from_str(
                        stone_colors_hidden[color as usize - 1],
                    ));
                    context.set_stroke_style(&JsValue::from_str(border_colors[color as usize - 1]));

                    draw_stone((x as _, y as _), size, true, true)?;
                }
            }
        }

        // Last stone marker //////////////////////////////////////////////////

        let last_stone = match (&game.state, &game.history) {
            (_, Some(h)) => h.last_stone.as_ref(),
            (GameStateView::Play(state), _) => state.last_stone.as_ref(),
            _ => None,
        };

        if let Some(points) = last_stone {
            for &(x, y) in points {
                let px = (x as i32 - self.board_displacement.0).rem_euclid(game.size.0 as i32);
                let py = (y as i32 - self.board_displacement.1).rem_euclid(game.size.1 as i32);
                let mut color = board[y as usize * game.size.0 as usize + x as usize];

                if color == 0 {
                    // White stones have the most fitting (read: black) marker for empty board
                    color = 2;
                }

                context.set_stroke_style(&JsValue::from_str(dead_mark_color[color as usize - 1]));
                context.set_line_width(2.0);

                draw_stone((px as _, py as _), size / 2., false, true)?;
            }
        }

        // States /////////////////////////////////////////////////////////////

        if game.history.is_none() {
            match &game.state {
                GameStateView::Scoring(scoring) | GameStateView::Done(scoring) => {
                    for group in &scoring.groups {
                        if group.alive {
                            continue;
                        }

                        for &(x, y) in &group.points {
                            let x = (x as i32 - self.board_displacement.0)
                                .rem_euclid(game.size.0 as i32);
                            let y = (y as i32 - self.board_displacement.1)
                                .rem_euclid(game.size.1 as i32);

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
                        let x = idx % board_size;
                        let y = idx / board_size;

                        let x = (x as i32 - self.board_displacement.0)
                            .rem_euclid(game.size.0 as i32) as f64;
                        let y = (y as i32 - self.board_displacement.1)
                            .rem_euclid(game.size.1 as i32) as f64;

                        if color.is_empty() {
                            continue;
                        }

                        context
                            .set_fill_style(&JsValue::from_str(stone_colors[color.0 as usize - 1]));

                        context.set_stroke_style(&JsValue::from_str(
                            border_colors[color.0 as usize - 1],
                        ));

                        context.fill_rect(
                            edge_size + (x + 1. / 3.) * size,
                            edge_size + (y + 1. / 3.) * size,
                            (1. / 3.) * size,
                            (1. / 3.) * size,
                        );
                    }
                }
                _ => {}
            }
        }

        let render_frame = self.link.callback(Msg::Render);
        let handle = RenderService::request_animation_frame(render_frame);

        // A reference to the new handle must be retained for the next render to run.
        self.render_loop = Some(Box::new(handle));

        Ok(())
    }
}
