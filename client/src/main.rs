#![allow(non_snake_case)]
mod config;
mod networking;
mod state;
mod window;

use dioxus::prelude::*;
use dioxus_router::prelude::*;
use dioxus_signals::Signal;
use shared::message::Profile;
use state::GameRoom;

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

    match mode {
        window::DisplayMode::Desktop => cx.render(rsx! {
            div {
                class: "root {mode.class()}",
                RoomList { rooms: state.read().rooms },
                GamePanel { id: *id }
            }
        }),
        window::DisplayMode::Mobile => cx.render(rsx! {
            div {
                class: "root {mode.class()}",
                GamePanel { id: *id }
            }
        }),
    }
}

#[component]
fn GamePanel(cx: Scope, id: u32) -> Element {
    cx.render(rsx! {
        div {
            "Game {id}"
        }
    })
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

            &.desktop {
                display: grid;
                grid-template-columns: 300px 1fr 100px;
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
