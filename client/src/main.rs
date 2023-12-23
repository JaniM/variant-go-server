#![allow(non_snake_case)]
mod config;
mod networking;
mod state;
mod window;

use std::rc::Rc;

use dioxus::{html::geometry::euclid::Size2D, prelude::*};
use dioxus_router::prelude::*;
use dioxus_signals::{use_selector, use_signal, ReadOnlySignal, Signal};
use shared::message::Profile;
use state::GameRoom;
use web_sys::wasm_bindgen::JsCast;

use crate::networking::use_websocket;

#[derive(Routable, Clone)]
enum Route {
    #[route("/")]
    #[redirect("/:.._segments", |_segments: Vec<String>| Route::Home {})]
    Home {},
    #[route("/game/:id")]
    GameRoute { id: u32 },
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
            class: "root {mode.class()}",
            RoomList { rooms: state.read().rooms },
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
            class: "root {mode.class()}",
            if mode.is_desktop() {
                rsx!(RoomList { rooms: state.read().rooms })
            }
            GamePanel { room: state.read().active_room() }
            if mode.is_desktop() {
                rsx!(div { style: "background: #242424;" })
            }
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
        canvas.set_width(div_size.width as u32);
        canvas.set_height(div_size.height as u32);

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
        let canvas = get_canvas();
        let context = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        context.begin_path();

        // Draw the outer circle.
        context
            .arc(75.0, 75.0, 50.0, 0.0, std::f64::consts::PI * 2.0)
            .unwrap();

        context.stroke();
    });

    #[rustfmt::skip]
    let class = sir::css!("
        width: 100%;
        height: 100%;
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

            &.desktop.small {
                display: grid;
                grid-template-columns: 0 1fr 300px;
            }

            &.mobile {
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

        a {
            display: flex;
            padding: 2px;
            background: #242424;
            cursor: pointer;
            color: var(--text-color);
            text-decoration: none;

            .mobile & {
                padding: 10px;
                border-bottom: 1px solid var(--text-color);
            }

            &:hover {
                background: #282828;
            }

            div:first-child {
                width: 50px;
                flex-shrink: 0;
                padding-right: 2px;
            }
        }
    ");
    cx.render(rsx! {
        ul {
            class: "{class} {mode.class()}",
            for room in rooms.iter() {
                Link {
                    to: Route::GameRoute { id: room.id },
                    key: "{room.id}",
                    div { "{room.id}" },
                    div { "{room.name}" },
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
