use yew::prelude::*;

use crate::game_view::Profile;
use crate::message::ClientMessage;
use crate::networking;
use crate::text_input::TextInput;

#[derive(Copy, Clone, PartialEq, Debug)]
enum Preset {
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
    oncreate: Callback<()>,
}

pub enum Msg {
    LoadPreset(Preset),
    SetName(String),
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
        CreateGameView {
            link,
            name: format!("{}'s game", props.user.nick_or("Unknown")),
            user: props.user,
            seats: vec![],
            komis: vec![],
            oncreate: props.oncreate,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::LoadPreset(preset) => {
                let (seats, komi) = match preset {
                    Preset::Standard => (vec![1, 2], vec![0, 15]),
                    Preset::Rengo2v2 => (vec![1, 2, 1, 2], vec![0, 15]),
                    Preset::ThreeColor => (vec![1, 2, 3], vec![0, 0, 0]),
                };
                self.seats = seats;
                self.komis = komi;
                true
            }
            Msg::SetName(name) => {
                self.name = name;
                true
            }
            Msg::OnCreate => {
                networking::send(ClientMessage::StartGame {
                    name: self.name.clone(),
                    seats: self.seats.clone(),
                    komis: self.komis.clone(),
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
