use dioxus::prelude::*;
use dioxus_signals::*;

use crate::state;
use shared::game::GameModifier;

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
    let modifiers = use_signal(cx, GameModifier::default);

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
                ModifierSelectors { modifiers: modifiers }
            }
        }
    })
}

#[component]
fn ModifierSelectors(cx: Scope, modifiers: Signal<GameModifier>) -> Element {
    let modifiers = *modifiers;

    #[rustfmt::skip]
    let class = sir::css!("
        padding: 10px;
        li {
            padding: 5px;
            label {
                cursor: pointer;
                margin-left: 5px;
            }
            .adjust {
                margin-left: 5px;
            }
        }
    ");

    cx.render(rsx! {
        ul {
            class: class,
            HiddenMoveGo { modifiers: modifiers }
            OneColorGo { modifiers: modifiers }
        }
    })
}

#[component]
fn OneColorGo(cx: Scope, modifiers: Signal<GameModifier>) -> Element {
    let flip_one_color = move || {
        let mut modifiers = modifiers.write();
        modifiers.visibility_mode = match modifiers.visibility_mode {
            Some(_) => None,
            None => Some(shared::game::VisibilityMode::OneColor),
        };
    };

    cx.render(rsx! {
        li {
            input {
                r#type: "checkbox",
                checked: modifiers.read().visibility_mode.is_some(),
                onclick: move |_| flip_one_color(),
            }
            label {
                class: "tooltip",
                onclick: move |_| flip_one_color(),
                "One color go"
                span {
                    class: "tooltip-text",
                    "Everyone sees the stones as same color. Confusion ensues."
                }
            }
        }
    })
}

#[component]
fn HiddenMoveGo(cx: Scope, modifiers: Signal<GameModifier>) -> Element {
    let hidden_move_placement_count = use_signal(cx, || 3);

    let flip_hidden_move = move || {
        let mut modifiers = modifiers.write();
        modifiers.hidden_move = match modifiers.hidden_move {
            Some(_) => None,
            None => Some(shared::game::HiddenMoveGo {
                placement_count: *hidden_move_placement_count.read(),
                teams_share_stones: true,
            }),
        };
    };

    cx.render(rsx! {
        li {
            input {
                r#type: "checkbox",
                checked: modifiers.read().hidden_move.is_some(),
                onclick: move |_| flip_hidden_move(),
            }
            label {
                class: "tooltip",
                onclick: move |_| flip_hidden_move(),
                "Hidden move go"
                span {
                    class: "tooltip-text",
                    "
Each team places stones before the game starts.
The opponents and viewers can't see their stones.
Stones are revealed if they cause a capture or prevent a move from being made.
If two players pick the same point, neither one gets a stone there, but they still see a marker for it."
                }
            }
            span {
                class: "adjust",
                "Placement stones: "
                input {
                    r#type: "number",
                    value: "{hidden_move_placement_count}",
                    onchange: move |e| hidden_move_placement_count.set(e.inner().value.parse().unwrap())
                }
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
