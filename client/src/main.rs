#![allow(non_snake_case)]
mod config;
mod networking;
mod state;
mod window;

use dioxus::prelude::*;
use dioxus_signals::Signal;
use shared::message::Profile;
use state::GameRoom;

use crate::networking::use_websocket;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    dioxus_web::launch(App);
}
fn App(cx: Scope) -> Element {
    let state = state::use_state_provider(&cx);
    global_style();

    window::use_window_size_provider(cx);
    let mode = window::use_display_mode(cx);

    #[rustfmt::skip]
    let class = sir::css!("
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
    ");
    cx.render(rsx! {
        sir::AppStyle {},
        div {
            class: "{class} {mode.class()}",
            RoomList { rooms: state.read().rooms },
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

        li {
            display: flex;
            padding: 2px;
            background: #242424;
            cursor: pointer;

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
                li {
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
