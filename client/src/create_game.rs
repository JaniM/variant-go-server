use web_sys::HtmlSelectElement;
use yew::prelude::*;

use crate::game::{self, GameModifier};
use crate::game_view::Profile;
use crate::message::ClientMessage;
use crate::networking;
use crate::text_input::TextInput;

#[derive(Copy, Clone, PartialEq, Debug)]
pub enum Preset {
    Standard,
    Rengo2v2,
    ThreeColor,
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
    SetHiddenMoveCount(u32),
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
            Msg::SetHiddenMoveCount(count) => {
                match &mut self.mods.hidden_move {
                    Some(rules) => {
                        rules.placement_count = count;
                    }
                    None => {}
                };
                true
            }
            Msg::OnCreate => {
                if self.seats.is_empty() || self.komis.is_empty() {
                    return false;
                }
                networking::send(ClientMessage::StartGame {
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
                let color = match team {
                    1 => "Black",
                    2 => "White",
                    3 => "Blue",
                    _ => "???",
                };

                let header = format!("Seat {} - {}", idx, color);

                html! { <li> {header} </li> }
            })
            .collect::<Html>();

        let komis = self
            .komis
            .iter()
            .enumerate()
            .map(|(idx, &amount)| {
                let color = match idx + 1 {
                    1 => "Black",
                    2 => "White",
                    3 => "Blue",
                    _ => "???",
                };

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
                <li><a href="#" onclick=self.link.callback(|_| Msg::LoadPreset(Preset::ThreeColor))>
                    {"Three color go"}
                </a></li>
            </ul>
        };

        let select_size = self.link.callback(|event| match event {
            ChangeData::Select(elem) => {
                let value = elem.selected_index();
                Msg::SelectSize(match value {
                    0 => 13,
                    1 => 19,
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
                <option value=13 selected=self.size == 13>{ "13" }</option>
                <option value=19 selected=self.size == 19>{ "19" }</option>
            </select>
        };

        let oncreate = self.link.callback(|_| Msg::OnCreate);

        html! {
            <div>
                <h2>{"Create game"}</h2>
                <span>
                    {"Name "}
                    <TextInput value=&self.name onsubmit=self.link.callback(Msg::SetName) />
                </span>
                <div>
                    {"Presets:"} {presets}
                    <span>{"Size:"} {size_selection}</span>
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
                            <label onclick=self.link.callback(move |_| Msg::ToggleHiddenMove)>{"Hidden move go"}</label>
                            {" Placement stones: "}
                            <input
                                style="width: 3em;"
                                type="number"
                                value={self.mods.hidden_move.as_ref().map(|x| x.placement_count).unwrap_or(0)}
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
                            <label onclick=self.link.callback(move |_| Msg::TogglePixel)>
                                {"Pixel go"}
                            </label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.zen_go.is_some()
                                onclick=self.link.callback(move |_| Msg::ToggleZen) />
                            <label onclick=self.link.callback(move |_| Msg::ToggleZen)>{"Zen go"}</label>
                        </li>
                        <li>
                            <input
                                type="checkbox"
                                class="toggle"
                                checked=self.mods.ponnuki_is_points.is_some()
                                onclick=self.link.callback(move |_| Msg::TogglePonnuki) />
                            <label onclick=self.link.callback(move |_| Msg::TogglePonnuki)>{"Ponnuki is 30 points"}</label>
                        </li>
                    </ul>
                </div>
                <div>
                    {"Seats:"}
                    <ul>{seats}</ul>
                </div>
                <div>
                    {"Komis:"}
                    <ul>{komis}</ul>
                </div>
                <button onclick=oncreate>{"Create"}</button>
            </div>
        }
    }
}
