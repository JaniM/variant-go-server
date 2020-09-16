use std::collections::HashMap;
use yew::prelude::*;

use crate::game::GameStateView;
use crate::game_view::*;
use crate::message;
use crate::networking;
use shared::game::Color;

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
            Msg::TakeSeat(idx) => networking::send(message::GameAction::TakeSeat(idx)),
            Msg::LeaveSeat(idx) => networking::send(message::GameAction::LeaveSeat(idx)),
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
            GameStateView::Scoring(state) | GameStateView::Done(state) => Some(&state.scores[..]),
            _ => Some(&self.props.game.points[..]),
        };

        let list = game
            .seats
            .iter()
            .enumerate()
            .map(|(idx, (occupant, color))| {
                let colorname = Color::name(*color);

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

                    let class = if game.turn == idx as u32 {
                        "occupied"
                    } else {
                        ""
                    };

                    let passed = match &game.state {
                        GameStateView::FreePlacement(state) if state.players_ready[idx] => {
                            " - ready!"
                        }
                        GameStateView::Play(state) if state.players_passed[idx] => " - passed!",
                        GameStateView::Scoring(state) if state.players_accepted[idx] => {
                            " - accepted!"
                        }
                        _ => "",
                    };

                    html! {
                        <div class=class style="margin: 5px 0;">
                            {format!("{}: {} {}{}", colorname, nick, scoretext, passed)}
                            {leave}
                        </div>
                    }
                } else {
                    html! {
                        <div style="margin: 5px 0;">
                            {format!("{}: unoccupied{}", colorname, scoretext)}
                            <button onclick=self.link.callback(move |_| Msg::TakeSeat(idx as _))>
                                {"Take seat"}
                            </button>
                        </div>
                    }
                }
            })
            .collect::<Html>();

        html!(<div style="margin: 10px;">{list}</div>)
    }
}
