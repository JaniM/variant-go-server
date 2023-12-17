#![allow(non_snake_case)]
mod config;
mod networking;
mod state;

use dioxus::prelude::*;
use dioxus_signals::Signal;
use shared::message::Profile;

use crate::networking::use_websocket;

fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    dioxus_web::launch(App);
}

fn App(cx: Scope) -> Element {
    let state = state::use_state_provider(&cx);
    cx.render(rsx! {
        div {
            "Hello, world!",
            NickInput { profile: state.read().user },
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
