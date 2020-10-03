use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::game_view::Profile;
use crate::message::StartGame;
use crate::networking;
use crate::text_input::TextInput;
use game::Color;
use shared::game::{self, GameModifier};

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Preset {
    Standard,
    Rengo2v2,
    ThreeColor,
    FourColor,
    ThreeColorRengo, // why?
}

pub struct CreateGameView {
    link: ComponentLink<Self>,
    name: String,
    user: Profile,
    seats: Vec<u8>,
    /// komi = amount/2 (for half)
    komis: Vec<i32>,
    size: u8,
    size_select_ref: NodeRef,
    oncreate: Callback<()>,
    mods: GameModifier,
}

pub enum Msg {
    LoadPreset(Preset),
    SelectSize(u8),
    SetName(String),
    TogglePixel,
    TogglePonnuki,
    ToggleZen,
    ToggleHiddenMove,
    ToggleOneColor,
    ToggleNoHistory,
    ToggleNPlusOne,
    ToggleCapturesGivePoints,
    ToggleTetris,
    SetHiddenMoveCount(u32),
    SetNPlusOneCount(u8),
    SetPonnukiValue(i32),
    OnCreate,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub user: Profile,
    pub oncreate: Callback<()>,
}

impl Component for CreateGameView {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let mut view = CreateGameView {
            link,
            name: format!("{}'s game", props.user.nick_or("Unknown")),
            user: props.user,
            seats: vec![],
            komis: vec![],
            size: 19,
            size_select_ref: NodeRef::default(),
            oncreate: props.oncreate,
            mods: GameModifier::default(),
        };
        view.update(Msg::LoadPreset(Preset::Standard));
        view
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::LoadPreset(preset) => {
                let (seats, komi, size) = match preset {
                    Preset::Standard => (vec![1, 2], vec![0, 15], 19),
                    Preset::Rengo2v2 => (vec![1, 2, 1, 2], vec![0, 15], 19),
                    Preset::ThreeColor => (vec![1, 2, 3], vec![0, 0, 0], 13),
                    Preset::FourColor => (vec![1, 2, 3, 4], vec![0, 0, 0, 0], 13),
                    Preset::ThreeColorRengo => (vec![1, 2, 3, 1, 2, 3], vec![0, 0, 0], 13),
                };
                self.seats = seats;
                self.komis = komi;
                self.size = size;
                // TODO: this is a hack
                self.mods.zen_go = None;
                if let Some(select) = self.size_select_ref.cast::<HtmlSelectElement>() {
                    select.set_value(&size.to_string());
                }
                true
            }
            Msg::SelectSize(size) => {
                self.size = size;
                true
            }
            Msg::SetName(name) => {
                self.name = name;
                true
            }
            Msg::TogglePixel => {
                self.mods.pixel = !self.mods.pixel;
                true
            }
            Msg::ToggleNoHistory => {
                self.mods.no_history = !self.mods.no_history;
                true
            }
            Msg::TogglePonnuki => {
                self.mods.ponnuki_is_points = match self.mods.ponnuki_is_points {
                    Some(_) => None,
                    // These are half points so 60 = 30 points
                    None => Some(60),
                };
                true
            }
            Msg::ToggleZen => {
                self.mods.zen_go = match &self.mods.zen_go {
                    None => {
                        self.seats.push(self.seats[0]);
                        Some(game::ZenGo {
                            color_count: self.komis.len() as u8,
                        })
                    }
                    Some(_) => {
                        self.seats.pop();
                        None
                    }
                };
                true
            }
            Msg::ToggleHiddenMove => {
                self.mods.hidden_move = match &self.mods.hidden_move {
                    None => Some(game::HiddenMoveGo {
                        placement_count: 5,
                        teams_share_stones: true,
                    }),
                    Some(_) => None,
                };
                true
            }
            Msg::ToggleNPlusOne => {
                self.mods.n_plus_one = match &self.mods.n_plus_one {
                    None => Some(game::NPlusOne { length: 4 }),
                    Some(_) => None,
                };
                true
            }
            Msg::SetHiddenMoveCount(count) => {
                match &mut self.mods.hidden_move {
                    Some(rules) => {
                        rules.placement_count = count;
                    }
                    None => {}
                };
                true
            }
            Msg::SetNPlusOneCount(count) => {
                match &mut self.mods.n_plus_one {
                    Some(rules) => {
                        rules.length = count;
                    }
                    None => {}
                };
                true
            }
            Msg::SetPonnukiValue(value) => {
                match &mut self.mods.ponnuki_is_points {
                    Some(rule) => {
                        *rule = value * 2;
                    }
                    None => {}
                };
                true
            }
            Msg::ToggleOneColor => {
                self.mods.visibility_mode = match self.mods.visibility_mode {
                    Some(game::VisibilityMode::OneColor) => None,
                    _ => Some(game::VisibilityMode::OneColor),
                };
                true
            }
            Msg::ToggleCapturesGivePoints => {
                self.mods.captures_give_points = match self.mods.captures_give_points {
                    Some(game::CapturesGivePoints {}) => None,
                    _ => Some(game::CapturesGivePoints {}),
                };
                true
            }
            Msg::ToggleTetris => {
                self.mods.tetris = match self.mods.tetris {
                    Some(game::TetrisGo {}) => None,
                    _ => Some(game::TetrisGo {}),
                };
                true
            }
            Msg::OnCreate => {
                if self.seats.is_empty() || self.komis.is_empty() {
                    return false;
                }
                networking::send(StartGame {
                    name: self.name.clone(),
                    seats: self.seats.clone(),
                    komis: self.komis.clone(),
                    size: (self.size, self.size),
                    mods: self.mods.clone(),
                });
                self.oncreate.emit(());
                false
            }
        }
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if props.user != self.user {
            self.name = format!("{}'s game", props.user.nick_or("Unknown"));
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let seats = self
            .seats
            .iter()
            .enumerate()
            .map(|(idx, team)| {
                let header = format!("Seat {} - {}", idx, Color::name(*team));

                html! { <li> {header} </li> }
            })
            .collect::<Html>();

        let komis = self
            .komis
            .iter()
            .enumerate()
            .map(|(idx, &amount)| {
                let color = Color::name(idx as u8 + 1);

                let header = format!("{}: {:.1}", color, amount as f32 / 2.);

                html! { <li> {header} </li> }
            })
            .collect::<Html>();

        let presets = html! {
            <ul>
                <li><a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::Standard))>
                    {"Standard"}
                </a></li>
                <li><a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::Rengo2v2))>
                    {"Rengo 2v2"}
                </a></li>
                <li>
                    <a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::ThreeColor))>
                        {"Three color go"}
                    </a>
                    {" / "}
                    <a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::FourColor))>
                        {"Four color go"}
                    </a>
                </li>
                <li><a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::ThreeColorRengo))>
                    {"Three color go (rengo)"}
                </a></li>
            </ul>
        };

        let select_size = self.link.callback(|event| match event {
            ChangeData::Select(elem) => {
                let value = elem.selected_index();
                Msg::SelectSize(match value {
                    0 => 9,
                    1 => 13,
                    2 => 19,
                    _ => unreachable!(),
                })
            }
            _ => unreachable!(),
        });

        let size_selection = html! {
            <select
                ref=self.size_select_ref.clone()
                onchange=select_size
            >
                <option value=9 selected=self.size == 9>{ "9" }</option>
                <option value=13 selected=self.size == 13>{ "13" }</option>
                <option value=19 selected=self.size == 19>{ "19" }</option>
            </select>
        };

        let oncreate = self.link.callback(|_| Msg::OnCreate);

        let tetris = html! {
            <li>
                <input
                    type="checkbox"
                    class="toggle"
                    checked=self.mods.tetris.is_some()
                    onclick=self.link.callback(move |_| Msg::ToggleTetris) />
                <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleTetris)>
                    {"Tetris go"}
                    <span class="tooltiptext">{"You can't play a group of exactly 4 stones. Diagonals don't form a group."}</span>
                </label>
            </li>
        };

        let options = html! {
            <div style="padding: 1em; flex-grow: 1;">
                <div>
                    {"Presets:"} {presets}
                    <span>{"Size: "} {size_selection}</span>
                </div>
                <div>
                    {"Modifiers"}
                    <ul>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.hidden_move.is_some()
                                onclick=self.link.callback(move |_| Msg::ToggleHiddenMove) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleHiddenMove)>
                                {"Hidden move go"}
                                <span class="tooltiptext">{r#"
Each team places stones before the game starts.
The opponents and viewers can't see their stones.
Stones are revealed if they cause a capture or prevent a move from being made.
If two players pick the same point, neither one gets a stone there, but they still see a marker for it."#}</span>
                            </label>
                            {" Placement stones: "}
                            <input
                                style="width: 3em;"
                                type="number"
                                value={self.mods.hidden_move.as_ref().map_or(5, |x| x.placement_count)}
                                disabled=self.mods.hidden_move.is_none()
                                onchange=self.link.callback(|data|
                                    match data {
                                        yew::events::ChangeData::Value(v) => Msg::SetHiddenMoveCount(v.parse().unwrap()),
                                        _ => unreachable!(),
                                    }
                                ) />
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.pixel
                                onclick=self.link.callback(move |_| Msg::TogglePixel) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::TogglePixel)>
                                {"Pixel go"}
                                <span class="tooltiptext">{"You place 2x2 blobs. Overlapping stones are ignored."}</span>
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.zen_go.is_some()
                                onclick=self.link.callback(move |_| Msg::ToggleZen) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleZen)>
                                {"Zen go"}
                                <span class="tooltiptext">{"One extra player. You get a different color on every turn. There are no winners."}</span>
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=matches!(self.mods.visibility_mode, Some(game::VisibilityMode::OneColor))
                                onclick=self.link.callback(move |_| Msg::ToggleOneColor) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleOneColor)>
                                {"One color go"}
                                <span class="tooltiptext">{"Everyone sees the stones as same color. Confusion ensues."}</span>
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.no_history
                                onclick=self.link.callback(move |_| Msg::ToggleNoHistory) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleNoHistory)>
                                {"No history (good for one color)"}
                                <span class="tooltiptext">{"No one can browse the past moves during the game."}</span>
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.n_plus_one.is_some()
                                onclick=self.link.callback(move |_| Msg::ToggleNPlusOne) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleNPlusOne)>
                                {"N+1"}
                                <span class="tooltiptext">{"You get an extra turn when you make a row of exactly N stones horizontally, vertically or diagonally."}</span>
                            </label>
                            {" "}
                            <input
                                style="width: 3em;"
                                type="number"
                                value={self.mods.n_plus_one.as_ref().map_or(4, |x| x.length)}
                                disabled=self.mods.n_plus_one.is_none()
                                onchange=self.link.callback(|data|
                                    match data {
                                        yew::events::ChangeData::Value(v) => Msg::SetNPlusOneCount(v.parse().unwrap()),
                                        _ => unreachable!(),
                                    }
                                ) />
                        </li>
                        {tetris}
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.captures_give_points.is_some()
                                onclick=self.link.callback(move |_| Msg::ToggleCapturesGivePoints) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::ToggleCapturesGivePoints)>
                                {"Captures give points"}
                                <span class="tooltiptext">{"Only the one to remove stones from the board gets the points. Promotes aggressive play. You only get points for removed stones, not dead stones in your territory."}</span>
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.ponnuki_is_points.is_some()
                                onclick=self.link.callback(move |_| Msg::TogglePonnuki) />
                            <label class="tooltip" onclick=self.link.callback(move |_| Msg::TogglePonnuki)>
                                {"Ponnuki is:"}
                                <span class="tooltiptext">{"Ponnuki requires a capture and all diagonals must be empty or different color"}</span>
                            </label>
                            {" "}
                            <input
                                style="width: 3em;"
                                type="number"
                                value={self.mods.ponnuki_is_points.map_or(30, |x| x/2)}
                                disabled=self.mods.ponnuki_is_points.is_none()
                                onchange=self.link.callback(|data|
                                    match data {
                                        yew::events::ChangeData::Value(v) => Msg::SetPonnukiValue(v.parse().unwrap()),
                                        _ => unreachable!(),
                                    }
                                ) />
                            {" points (can be negative)"}
                        </li>
                    </ul>
                </div>
            </div>
        };

        html! {
            <div style="flex-grow: 1; margin: 10px; display: flex; justify-content: center;">
            <div style="width: 800px; word-break: normal; margin: auto 0;">
                <h2>{"Create game"}</h2>
                <span>
                    {"Name "}
                    <TextInput value=&self.name onsubmit=self.link.callback(Msg::SetName) />
                </span>
                <div style="display: flex;">
                    {options}
                    <div style="border-left: 1px solid #dedede; padding: 1em;">
                        <div>
                            {"Seats:"}
                            <ul>{seats}</ul>
                        </div>
                        <div>
                            {"Komis:"}
                            <ul>{komis}</ul>
                        </div>
                    </div>
                </div>
                <button onclick=oncreate>{"Create"}</button>
                <div>
                <p>
                    {r#"All game modes use area scoring (i.e. neutral intersections are worth points) and positional superko (board state can never repeat).
                        In three-colour and four-colour go, a player who captures stones gets no advantage over a player who didn't, but the player whose stones are captured loses points.
                        These rules are close to Chinese rules, just generalized by virtue of superko."#}
                </p>
                </div>
            </div>
            </div>
        }
    }
}
