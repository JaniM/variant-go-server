#![allow(non_snake_case)]
mod board;
mod config;
mod networking;
mod palette;
mod state;
mod views;
mod window;

use std::rc::Rc;

use dioxus::{
    html::{geometry::euclid::Size2D, input_data::MouseButton},
    prelude::*,
};
use dioxus_router::prelude::*;
use dioxus_signals::{use_selector, use_signal, ReadOnlySignal, Signal};
use shared::{game::Seat, message::Profile};
use state::GameRoom;
use web_sys::wasm_bindgen::JsCast;
use window::DisplayMode;

use crate::{board::Board, state::ActionSender};

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    #[redirect("/:.._segments", |_segments: Vec<String>| Route::Home {})]
    Home {},
    #[route("/game/:id")]
    GameRoute { id: u32 },
    #[route("/create")]
    CreateRoute {},
}

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    dioxus_web::launch(App);
}

fn App(cx: Scope) -> Element {
    global_style();
    state::use_state_provider(&cx);
    window::use_window_size_provider(cx);
    cx.render(rsx! {
        sir::AppStyle {},
        Router::<Route> {},
    })
}

#[component]
fn Home(cx: Scope) -> Element {
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);

    cx.render(rsx! {
        div {
            class: "root {mode.class()} in-list",
            RoomList { rooms: state.read().rooms },
            div {},
            div {}
        }
    })
}

#[component]
fn GameRoute(cx: Scope, id: u32) -> Element {
    let action = ActionSender::new(cx);
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);

    let _ = use_memo(cx, (id,), move |(id,)| {
        action.join_room(id);
    });

    cx.render(rsx! {
        div {
            class: "root {mode.class()} in-game",
            if mode.is_desktop() {
                rsx!(RoomList { rooms: state.read().rooms })
            }
            div {
                class: "center-stack",
                GameNavBar { room: state.read().active_room() },
                if mode.is_mobile() {
                    rsx!(SeatCards {})
                }
                GamePanel { room: state.read().active_room() }
            }
            if mode.is_desktop() {
                rsx!(RightPanel {})
            }
        }
    })
}

#[component]
fn CreateRoute(cx: Scope) -> Element {
    let action = ActionSender::new(cx);
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);

    // We only care about room events if GameRoute is active.
    use_on_create(cx, move || {
        action.leave_all_rooms();
        async {}
    });

    use_game_switcher(cx);

    cx.render(rsx! {
        div {
            class: "root {mode.class()} in-game",
            if mode.is_desktop() {
                rsx!(RoomList { rooms: state.read().rooms })
            }
            div {
                class: "center-stack",
                CreateGameNavBar {},
                views::CreateGamePanel { }
            }
            if mode.is_desktop() {
                rsx!(RightPanel {})
            }
        }
    })
}

/// Switches to a room automatically if the active room changes.
/// This is necessary as we don't know which room to switch to when we create a game.
/// Regrets with the protocol, I have a few.
fn use_game_switcher(cx: &ScopeState) {
    let state = state::use_state(cx);
    let navigator = use_navigator(cx);
    let room_previous = use_signal(cx, || None::<u32>);
    let room_current = dioxus_signals::use_selector(cx, move || {
        let state = state.read();
        state.active_room().read().as_ref().map(|r| r.id)
    });

    if let Some(room) = *room_current.read() {
        if Some(room) != *room_previous.read() {
            navigator.push(Route::GameRoute { id: room });
        }
        room_previous.set(Some(room));
    } else if room_previous.read().is_some() {
        room_previous.set(None);
    };
}

#[component]
fn RightPanel(cx: Scope) -> Element {
    #[rustfmt::skip]
    let class = sir::css!("
        background: #242424;
    ");
    cx.render(rsx! {
        div {
            class: "{class}",
            SeatCards {}
        }
    })
}

#[component]
fn GamePanel(cx: Scope, room: ReadOnlySignal<Option<state::ActiveRoom>>) -> Element {
    let outer_div = use_signal(cx, || None::<Rc<MountedData>>);
    let canvas_element = use_signal(cx, || None::<Rc<MountedData>>);
    let size = use_signal(cx, Size2D::default);
    let set_size = move || async move {
        let Some(data) = outer_div.read().clone() else {
            return;
        };
        let rect = data.get_client_rect().await.unwrap_or_default();
        let mut div_size = rect.size;
        div_size.width = f64::min(div_size.width, div_size.height);
        div_size.height = f64::min(div_size.width, div_size.height);

        // Resize the canvas instantly to allow rendering
        let canvas = get_canvas();
        let pixel_ratio = gloo_utils::window().device_pixel_ratio();
        let unscaled = div_size.width as u32;
        let scaled = (div_size.width * pixel_ratio) as u32;
        canvas.set_width(scaled);
        canvas.set_height(scaled);
        canvas
            .style()
            .set_property("width", &format!("{}px", unscaled))
            .unwrap();
        canvas
            .style()
            .set_property("height", &format!("{}px", unscaled))
            .unwrap();

        size.set(div_size);
    };
    let onmounted = move |e: MountedEvent| {
        let data = e.inner().clone();
        outer_div.set(Some(data));
        cx.spawn(set_size());
    };
    let window_size = window::use_window_size(cx);
    use_effect(cx, (&window_size,), move |_| set_size());

    let room = *room;
    let view =
        dioxus_signals::use_selector(cx, move || room.read().as_ref().map(|r| r.view.clone()));
    let board = dioxus_signals::use_signal(cx, || Board {
        palette: palette::PaletteOption::get().to_palette(),
        toroidal_edge_size: 0, // TODO: Implement toroidal edges
        board_displacement: (0, 0),
        selection_pos: None,
        input: board::Input::None,
        show_hidden: false,
        edge_size: 40.0,
    });

    dioxus_signals::use_effect(cx, move || {
        // Subacribe to size changes
        let _ = size.read();
        let Some(_mount_data) = canvas_element.read().clone() else {
            return;
        };
        let Some(view) = view.read().clone() else {
            return;
        };
        let canvas = get_canvas();
        let board = board.read();
        board.render_gl(&canvas, &*view, None).unwrap();
    });

    let action = ActionSender::new(cx);

    let update_mouse = move |e: MouseEvent, clicked: bool| {
        let Some(view) = view.read().clone() else {
            return;
        };
        let canvas = get_canvas();
        let coord = e.client_coordinates();
        let bounding_rect = canvas.get_bounding_client_rect();
        let mut board = board.write();
        let input =
            board::Input::from_pointer(&board, &view, coord.to_tuple(), bounding_rect, clicked);
        board.input = input;
        board.selection_pos = input.into_selection();

        if let board::Input::Place(pos, true) = input {
            action.place_stone(pos.0, pos.1);
        }
    };

    let on_click = move |e: MouseEvent| {
        let clicked = e.held_buttons().contains(MouseButton::Primary);
        update_mouse(e, clicked);
    };

    #[rustfmt::skip]
    let class = sir::css!("
        width: 100%;
        height: 100%;
        display: flex;
        justify-content: center;
        canvas {
            position: absolute;
        }
    ");
    cx.render(rsx! {
        div {
            class: "{class}",
            onmounted: onmounted,
            canvas {
                onmounted: move |e| {
                    canvas_element.set(Some(e.inner().clone()));
                },
                onmousemove: move |e| update_mouse(e, false),
                onmousedown: move |e| on_click(e),
                id: "game-canvas",
            }
        }
    })
}

#[component]
fn CreateGameNavBar(cx: Scope) -> Element {
    let mode = window::use_display_mode(cx);

    #[rustfmt::skip]
    let class = sir::css!("
        display: flex;

        a {
            display: flex;
            background: #242424;
            cursor: pointer;
            color: var(--text-color);
            text-decoration: none;

            flex-grow: 0;

            padding: 10px;

            &:not(:last-child) {
                border-right: 1px solid var(--text-color);
            }

            &:hover {
                background: #282828;
            }
        }
    ");

    cx.render(rsx! {
        div {
            class: "{class}",
            if !mode.is_large_desktop() {
                rsx!(Link {
                    to: Route::Home {},
                    "↩ Game List"
                })
            }
        }
    })
}

#[component]
fn GameNavBar(cx: Scope, room: ReadOnlySignal<Option<state::ActiveRoom>>) -> Element {
    let mode = window::use_display_mode(cx);
    let state = state::use_state(cx);
    let room = *room;
    let view =
        dioxus_signals::use_selector(cx, move || room.read().as_ref().map(|r| r.view.clone()));

    #[rustfmt::skip]
    let class = sir::css!("
        display: flex;
        height: 40px;
        
        &.desktop {
            padding-left: 10px;
            padding-right: 10px;
        }

        a {
            display: flex;
            background: #242424;
            cursor: pointer;
            color: var(--text-color);
            text-decoration: none;

            flex-grow: 0;

            padding: 10px;

            &:not(:last-child) {
                border-right: 1px solid var(--text-color);
            }

            &:hover {
                background: #282828;
            }
        }

        .pad {
            flex-grow: 1;
        }
    ");

    #[derive(Copy, Clone, Default, PartialEq)]
    struct Info {
        is_own_turn: bool,
        is_play: bool,
    }

    let Info {
        is_own_turn,
        is_play,
    } = *dioxus_signals::use_selector(cx, move || {
        let view = view.read();
        let Some(view) = view.as_ref() else {
            return Info::default();
        };
        let me = state.read().user.read().user_id;
        let seat = &view.seats[view.turn as usize];
        Info {
            is_own_turn: seat.player == Some(me),
            is_play: matches!(view.state, shared::game::GameStateView::Play(_)),
        }
    })
    .read();

    let action = ActionSender::new(cx);

    cx.render(rsx! {
        div {
            class: "{class} {mode.class()}",
            if !mode.is_large_desktop() {
                rsx!(Link {
                    to: Route::Home {},
                    "↩ Game List"
                })
            }
            div { class: "pad" }
            if is_own_turn && is_play {
                rsx!(a {
                    onclick: move |_| action.undo(),
                    "Undo"
                })
            }
            if is_own_turn && is_play {
                rsx!(a {
                    onclick: move |_| action.pass(),
                    "Pass"
                })
            }
        }
    })
}

#[component]
fn SeatCards(cx: Scope) -> Element {
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);
    let room = state.read().active_room();
    let seats = use_selector(cx, move || Some(room.read().as_ref()?.view.seats.clone()));

    #[rustfmt::skip]
    let class = sir::css!("
        &.desktop {
            display: grid;
        }
        &.mobile {
            display: grid;
        }
    ");

    let columns = match mode {
        DisplayMode::Mobile => format!(
            "grid-template-columns:repeat({}, 1fr);",
            seats.read().iter().flatten().count()
        ),
        DisplayMode::Desktop(_) => "grid-template-columns:1fr;".to_owned(),
    };

    cx.render(rsx! {
        div {
            class: "{class} {mode.class()}",
            style: "{columns}",
            for (id, seat) in seats.read().iter().flatten().enumerate() {
                SeatCard {
                    seat: seat.clone(),
                    seat_id: id as u32,
                }
            }
        }
    })
}

#[component]
fn SeatCard(cx: Scope, seat: Seat, seat_id: u32) -> Element {
    let seat = *seat;
    let seat_id = *seat_id;
    let action = ActionSender::new(cx);

    let state = state::use_state(cx);
    let profiles = state.read().profiles;

    // TODO: Switch to this when the selector escape bug is fixed
    // See https://github.com/DioxusLabs/dioxus/issues/1745
    // let profile = use_selector_with_dependencies(cx, seat, {
    //     move |seat| profiles.read().get(&seat.player?).cloned()
    // });
    let profile = {
        let profiles = profiles.read();
        seat.player.and_then(|p| profiles.get(&p)).cloned()
    };

    let nick = profile
        .as_ref()
        .map(|p| p.nick.as_deref().unwrap_or("Unknown"));

    let held_hy_self = seat
        .player
        .map_or(false, |p| p == state.read().user.read().user_id);

    let palette = palette::PaletteOption::get().to_palette();
    let bg_color = palette.stone_colors[seat.team.as_usize() - 1];
    let fg_color = palette.dead_mark_color[seat.team.as_usize() - 1];

    #[rustfmt::skip]
    let class = sir::css!("
        background: var(--bg-color);
        color: var(--fg-color);

        display: grid;
        grid-template-columns: 1fr auto;

        div {
            padding: 10px;
            display: flex;
            align-items: center;
        }

        button {
            height: 100%;
            padding: 10px;
            background: var(--bg-color);
            filter: brightness(80%);
            color: var(--fg-color);
            border: 1px solid var(--highlight-color);

            cursor: pointer;

            &:hover {
                filter: brightness(100%);
            }
        }
    ");

    let take_seat = move || {
        action.take_seat(seat_id);
    };

    let leave_seat = move || {
        action.leave_seat(seat_id);
    };

    cx.render(rsx! {
        div {
            class: "{class}",
            style: "--bg-color: {bg_color}; --fg-color: {fg_color};",
            div {
                if let Some(nick) = nick {
                    rsx!("{nick}")
                } else if seat.player.is_none() {
                    rsx!("<empty>")
                }
            }
            if seat.player.is_none() {
                rsx!(button {
                    onclick: move |_| take_seat(),
                    "Take Seat"
                })
            } else if held_hy_self {
                rsx!(button {
                    onclick: move |_| leave_seat(),
                    "Leave Seat"
                })
            }
        }
    })
}

fn get_canvas() -> web_sys::HtmlCanvasElement {
    let canvas = gloo_utils::document()
        .get_element_by_id("game-canvas")
        .unwrap();
    canvas
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .map_err(|_| ())
        .unwrap()
}

fn global_style() {
    #[rustfmt::skip]
    sir::global_css!("
        * {
            margin: 0;
            padding: 0;
        }
        body, #main {
            --bg-color: #222;
            --bg-h-color: #444;
            --text-color: #eee;
            width: 100vw;
            height: 100vh;
            overflow: hidden;
        }
        .root {
            height: 100%;
            overflow: hidden;
            background: var(--bg-color);
            color: var(--text-color);

            &.desktop.large {
                display: grid;
                grid-template-columns: 300px 1fr 300px;
            }

            &.desktop.small.in-game {
                display: grid;
                grid-template-columns: 0 1fr 300px;
            }

            &.desktop.small.in-list {
                display: grid;
                grid-template-columns: 300px 1fr 0;
            }

            &.desktop .center-stack {
                height: 100%;
                display: grid;
                grid-template-rows: auto 1fr;
            }

            &.mobile {
            }

            &.mobile .center-stack {
                height: 100%;
                display: grid;
                grid-template-rows: auto auto 1fr;
            }
        }

        .tooltip {
            position: relative;
            border-bottom: 1px dotted var(--text-color);

            /* Tooltip text */
            .tooltip-text {
                visibility: hidden;
                width: 25em;
                background-color: #363532;
                color: #dedede;
                text-align: center;
                padding: 5px 5px;
                border-radius: 6px;

                position: absolute;
                z-index: 1;

                top: -4px; left: 105%;
                opacity: 0;
                transition: opacity 0.5s;
            }

            &:hover .tooltip-text {
                visibility: visible;
                opacity: 1;
            }

        }
    ");
}

#[component]
fn RoomList(cx: Scope, rooms: Signal<Vec<GameRoom>>) -> Element {
    let rooms = rooms.read();
    let mode = window::use_display_mode(cx);
    #[rustfmt::skip]
    let class = sir::css!("
        height: 100%;
        overflow-y: scroll;

        .actions {
            mzrgin-bottom: 10px;
            a { padding: 10px; }
        }

        a {
            display: flex;
            padding: 2px;
            background: #242424;
            cursor: pointer;
            color: var(--text-color);
            text-decoration: none;

            .mobile &.game {
                padding: 10px;
                border-bottom: 1px solid var(--text-color);
            }

            &:hover, &:focus {
                background: #282828;
            }

            &.game div:first-child {
                width: 50px;
                flex-shrink: 0;
                padding-right: 2px;
            }
        }
    ");
    cx.render(rsx! {
        div {
            class: "{class} {mode.class()}",
            div {
                class: "actions",
                Link {
                    to: Route::CreateRoute {},
                    div { "Create Game" },
                }
            }
            ul {
                for room in rooms.iter() {
                    Link {
                        class: "game",
                        to: Route::GameRoute { id: room.id },
                        key: "{room.id}",
                        div { "{room.id}" },
                        div { "{room.name}" },
                    }
                }
            }
        }
    })
}

#[component]
fn NickInput(cx: Scope, profile: Signal<Profile>) -> Element {
    let action = ActionSender::new(cx);
    let profile = profile.read();
    let nick = profile.nick.as_deref().unwrap_or("");
    let on_change = move |e: FormEvent| {
        let nick = &e.inner().value;
        action.set_nick(nick);
    };
    cx.render(rsx! {
        input {
            value: "{nick}",
            onchange: on_change,
        }
    })
}
