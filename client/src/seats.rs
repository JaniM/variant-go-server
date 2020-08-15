use std::collections::HashMap;
use yew::prelude::*;

use crate::game::GameState;
use crate::game_view::*;
use crate::message::{self, ClientMessage};
use crate::networking;

pub struct SeatList {
    link: ComponentLink<Self>,
    props: Props,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub game: GameView,
    pub profiles: HashMap<u64, Profile>,
    pub user: Option<Profile>,
}

pub enum Msg {
    TakeSeat(u32),
    LeaveSeat(u32),
}

impl Component for SeatList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        SeatList { link, props }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::TakeSeat(idx) => networking::send(ClientMessage::GameAction(
                message::GameAction::TakeSeat(idx),
            )),
            Msg::LeaveSeat(idx) => networking::send(ClientMessage::GameAction(
                message::GameAction::LeaveSeat(idx),
            )),
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        let game = &self.props.game;
        let scores = match &self.props.game.state {
            GameState::Scoring(state) | GameState::Done(state) => Some(&state.scores),
            _ => None,
        };

        let list = game
            .seats
            .iter()
            .enumerate()
            .map(|(idx, (occupant, color))| {
                let colorname = match color {
                    1 => "Black",
                    2 => "White",
                    _ => "???",
                };

                let scoretext = match scores {
                    Some(scores) => {
                        format!(" - Score: {:.1}", scores[*color as usize - 1] as f32 / 2.)
                    }
                    None => "".to_owned(),
                };

                if let Some(id) = occupant {
                    let nick = self
                        .props
                        .profiles
                        .get(id)
                        .and_then(|p| p.nick.as_ref())
                        .map(|n| &**n)
                        .unwrap_or("no nick");
                    let leave = if self.props.user.as_ref().map(|x| x.user_id) == Some(*id) {
                        html!(<button onclick=self.link.callback(move |_| Msg::LeaveSeat(idx as _))>
                        {"Leave seat"}
                    </button>)
                    } else {
                        html!()
                    };

                    let style = if game.turn == idx as u32 {
                        "background-color: #eeeeee;"
                    } else {
                        ""
                    };
                    let passed = match &game.state {
                        GameState::Play(state) if state.players_passed[idx] => " - passed!",
                        GameState::Scoring(state) if state.players_accepted[idx] => " - accepted!",
                        _ => "",
                    };

                    html! {
                        <li style=style>
                            {format!("{}: {} {}{}", colorname, nick, scoretext, passed)}
                            {leave}
                        </li>
                    }
                } else {
                    html! {
                        <li>
                            {format!("{}: unoccupied{}", colorname, scoretext)}
                            <button onclick=self.link.callback(move |_| Msg::TakeSeat(idx as _))>
                                {"Take seat"}
                            </button>
                        </li>
                    }
                }
            })
            .collect::<Html>();

        html!(<ul>{list}</ul>)
    }
}
