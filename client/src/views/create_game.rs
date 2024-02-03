use dioxus::prelude::*;
use dioxus_signals::*;

use crate::state;
use shared::game::GameModifier;

macro_rules! simple_modifier {
    ($name:ident, $modifiers:ident => $select:expr, $flip:expr, $text:expr, $tooltip:expr) => {
        #[component]
        fn $name(cx: Scope, modifiers: Signal<GameModifier>) -> Element {
            let flip = move || {
                let mut $modifiers = modifiers.write();
                $flip;
            };

            cx.render(rsx! {
                li {
                    input {
                        r#type: "checkbox",
                        checked: {
                            let $modifiers = modifiers.read();
                            $select
                        },
                        onclick: move |_| flip(),
                    }
                    label {
                        class: "tooltip",
                        onclick: move |_| flip(),
                        $text
                        span {
                            class: "tooltip-text",
                            $tooltip
                        }
                    }
                }
            })
        }
    };
}

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
            PixelGo { modifiers: modifiers }
            ZenGo { modifiers: modifiers }
            OneColorGo { modifiers: modifiers }
            NoHistory { modifiers: modifiers }
            TetrisGo { modifiers: modifiers }
            ToroidalGo { modifiers: modifiers }
            PhantomGo { modifiers: modifiers }
            CapturesGivePoints { modifiers: modifiers }
            Observable { modifiers: modifiers }
            NoUndo { modifiers: modifiers }
        }
    })
}

// TODO: N+1
// TODO: Traitor go Traitor stones:
// TODO: Ponnuki is: points (can be negative)

simple_modifier!(
    OneColorGo,
    modifiers => modifiers.visibility_mode.is_some(),
    modifiers.visibility_mode = match modifiers.visibility_mode {
        Some(_) => None,
        None => Some(shared::game::VisibilityMode::OneColor),
    },
    "One color go",
    "Everyone sees the stones as same color. Confusion ensues."
);

simple_modifier!(
    PixelGo,
    modifiers => modifiers.pixel,
    modifiers.pixel = !modifiers.pixel,
    "Pixel go",
    "You place 2x2 blobs. Overlapping stones are ignored."
);

// TODO: Ensure zen go receives the correct color count from the preset
simple_modifier!(
    ZenGo,
    modifiers => modifiers.zen_go.is_some(),
    modifiers.zen_go = match modifiers.zen_go {
        Some(_) => None,
        None => Some(shared::game::ZenGo::default()),
    },
    "Zen go",
    "One extra player. You get a different color on every turn. There are no winners."
);

simple_modifier!(
    NoHistory,
    modifiers => modifiers.no_history,
    modifiers.no_history = !modifiers.no_history,
    "No history (good for one color)",
    "No one can browse the past moves during the game."
);

simple_modifier!(
    TetrisGo,
    modifiers => modifiers.tetris.is_some(),
    modifiers.tetris = match modifiers.tetris {
        Some(_) => None,
        None => Some(shared::game::TetrisGo {}),
    },
    "Tetris go",
    "You can't play a group of exactly 4 stones. Diagonals don't form a group."
);

simple_modifier!(
    ToroidalGo,
    modifiers => modifiers.toroidal.is_some(),
    modifiers.toroidal = match modifiers.toroidal {
        Some(_) => None,
        None => Some(shared::game::ToroidalGo {}),
    },
    "Toroidal go",
    "Opposing edges are connected. First line doesn't exist. Click on the borders, shift click on a point or use WASD or 8462 to move the view. Use < and > or + and - to adjust the extended view."
);

simple_modifier!(
    PhantomGo,
    modifiers => modifiers.phantom.is_some(),
    modifiers.phantom = match modifiers.phantom {
        Some(_) => None,
        None => Some(shared::game::PhantomGo {}),
    },
    "Phantom go",
    "All stones are invisible when placed. They become visible when they affect the game (like hidden move go). Atari also reveals."
);

simple_modifier!(
    CapturesGivePoints,
    modifiers => modifiers.captures_give_points.is_some(),
    modifiers.captures_give_points = match modifiers.captures_give_points {
        Some(_) => None,
        None => Some(shared::game::CapturesGivePoints {}),
    },
    "Captures give points",
    "Only the one to remove stones from the board gets the points. Promotes aggressive play. You only get points for removed stones, not dead stones in your territory."
);

simple_modifier!(
    Observable,
    modifiers => modifiers.observable,
    modifiers.observable = !modifiers.observable,
    "Observable",
    "All users who are not holding a seat can see all hidden stones and the true color of stones if one color go is enabled."
);

simple_modifier!(
    NoUndo,
    modifiers => modifiers.no_undo,
    modifiers.no_undo = !modifiers.no_undo,
    "Undo not allowed",
    "Disables undo for all players."
);

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
