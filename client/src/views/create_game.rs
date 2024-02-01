use dioxus::prelude::*;
use dioxus_signals::*;

use crate::state;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Preset {
    Standard,
    Rengo,
    ThreeColor,
    FourColor,
    ThreeColorRengo,
}

impl Preset {
    fn name(self) -> &'static str {
        match self {
            Preset::Standard => "Standard",
            Preset::Rengo => "Rengo",
            Preset::ThreeColor => "Three Color Go",
            Preset::FourColor => "Four Color Go",
            Preset::ThreeColorRengo => "Three Color Rengo",
        }
    }
}

#[component]
pub fn CreateGamePanel(cx: Scope) -> Element {
    let state = state::use_state(cx);
    let game_name = use_signal(cx, || {
        format!("{}'s game", state::username(&state.read().user.read()))
    });
    let chosen_preset = use_signal(cx, || Preset::Standard);

    #[rustfmt::skip]
    let class = sir::css!("
        display: grid;
        grid-template-columns: 1fr 1fr;
        padding: 20px;
    ");
    cx.render(rsx! {
        div {
            class: class,
            div {
                NameInput { name: game_name }
                PresetSelectors { chosen_preset: chosen_preset }
            }
        }
    })
}

#[component]
fn PresetSelectors(cx: Scope, chosen_preset: Signal<Preset>) -> Element {
    let presets = [
        Preset::Standard,
        Preset::Rengo,
        Preset::ThreeColor,
        Preset::FourColor,
        Preset::ThreeColorRengo,
    ];

    #[rustfmt::skip]
    let class = sir::css!("
        padding: 10px;
        li {
            cursor: pointer;
            width: 200px;
            padding: 5px;
            &.active {
                background-color: var(--bg-h-color);
            }
        }
    ");

    cx.render(rsx! {
        ul {
            class: class,
            for preset in presets {
                li {
                    class: if preset == *chosen_preset.read() { "active" } else { "" },
                    onclick: move |_| chosen_preset.set(preset),
                    preset.name()
                }
            }
        }
    })
}

#[component]
fn NameInput(cx: Scope, name: Signal<String>) -> Element {
    #[rustfmt::skip]
    let class = sir::css!("
        label {
            margin-right: 5px;
        }
    ");

    cx.render(rsx! {
        div {
            class: class,
            label { "Game name" }
            input {
                r#type: "text",
                value: "{name}",
                oninput: move |e| name.set(e.value.clone()),
            }
        }
    })
}
