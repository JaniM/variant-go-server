#![allow(non_snake_case)]
mod board;
mod config;
mod networking;
mod palette;
mod state;
mod views;
mod window;

use std::{collections::HashMap, rc::Rc};

use dioxus::{html::geometry::euclid::Size2D, prelude::*};
use dioxus_router::prelude::*;
use dioxus_signals::{
    use_selector, use_selector_with_dependencies, use_signal, ReadOnlySignal, Signal,
};
use shared::{game::Seat, message::Profile};
use state::GameRoom;
use web_sys::wasm_bindgen::JsCast;
use window::DisplayMode;

use crate::{board::Board, networking::use_websocket};

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
    let send = use_websocket(cx);
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);

    let _ = use_memo(cx, (id,), move |(id,)| {
        send(state::join_room(id));
    });

    cx.render(rsx! {
        div {
            class: "root {mode.class()} in-game",
            if mode.is_desktop() {
                rsx!(RoomList { rooms: state.read().rooms })
            }
            div {
                class: "center-stack",
                GameNavBar {},
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
    let send = use_websocket(cx);
    let state = state::use_state(cx);
    let mode = window::use_display_mode(cx);

    // We only care about room events if GameRoute is active.
    use_on_create(cx, move || {
        send(state::leave_all_rooms());
        async {}
    });

    cx.render(rsx! {
        div {
            class: "root {mode.class()} in-game",
            if mode.is_desktop() {
                rsx!(RoomList { rooms: state.read().rooms })
            }
            div {
                class: "center-stack",
                GameNavBar {},
                views::CreateGamePanel { }
            }
            if mode.is_desktop() {
                rsx!(RightPanel {})
            }
        }
    })
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

    dioxus_signals::use_effect(cx, move || {
        // Subacribe to size changes
        let _ = size.read();
        let Some(mount_data) = canvas_element.read().clone() else {
            return;
        };
        let Some(view) = view.read().clone() else {
            return;
        };
        let canvas = get_canvas();
        let board = Board {
            palette: palette::PaletteOption::get().to_palette(),
            toroidal_edge_size: if view.mods.toroidal.is_some() { 3 } else { 0 },
            board_displacement: (0, 0),
            selection_pos: None,
            input: board::Input::None,
            show_hidden: false,
        };
        board.render_gl(&canvas, &*view, None).unwrap();
    });

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
                id: "game-canvas",
            }
        }
    })
}

#[component]
fn GameNavBar(cx: Scope) -> Element {
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
            a {
                "wow"
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
            for seat in seats.read().iter().flatten() {
                SeatCard {
                    seat: seat.clone()
                }
            }
        }
    })
}

#[component]
fn SeatCard(cx: Scope, seat: Seat) -> Element {
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

    let palette = palette::PaletteOption::get().to_palette();
    let bg_color = palette.stone_colors[seat.team.as_usize() - 1];
    let fg_color = palette.dead_mark_color[seat.team.as_usize() - 1];

    #[rustfmt::skip]
    let class = sir::css!("
        padding: 10px;
    ");

    cx.render(rsx! {
        div {
            class: "{class}",
            style: "background: {bg_color}; color: {fg_color};",
            div {
                if let Some(nick) = nick {
                    rsx!("{nick}")
                } else if seat.player.is_none() {
                    rsx!("<empty>")
                }
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
    let send = use_websocket(cx);
    let profile = profile.read();
    let nick = profile.nick.as_deref().unwrap_or("");
    let on_change = move |e: FormEvent| {
        send(state::set_nick(&e.inner().value));
    };
    cx.render(rsx! {
        input {
            value: "{nick}",
            onchange: on_change,
        }
    })
}