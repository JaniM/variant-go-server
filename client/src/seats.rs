use js_sys::Date;
use std::collections::HashMap;
use std::time::Duration;
use yew::prelude::*;

use crate::game::GameStateView;
use crate::game_view::*;
use crate::message;
use crate::networking;
use shared::game::clock::PlayerClock;
use shared::game::Color;

use crate::if_html;

use yew::services::interval::{IntervalService, IntervalTask};

struct Audio {
    time_sound: Option<web_sys::HtmlAudioElement>,
    /// Number of milliseconds since epoch
    last_play: f64,
}

impl Audio {
    fn new() -> Audio {
        let path = "/sounds/countdownbeep.wav";

        let time_sound = web_sys::HtmlAudioElement::new_with_src(path).ok();

        Audio {
            time_sound,
            last_play: Date::now(),
        }
    }

    fn play_beep(&mut self) {
        let time = Date::now();
        if time - self.last_play < 6000.0 {
            return;
        }
        self.last_play = time;

        if let Some(sound) = &self.time_sound {
            sound.set_current_time(0.0);
            // TODO: PUZZLE unhardcode this
            sound.set_volume(0.50);
            let _ = sound.play();
        }
    }

    fn stop_beep(&mut self) {
        self.last_play = 0.0;

        if let Some(sound) = &self.time_sound {
            let _ = sound.pause();
        }
    }
}

pub struct SeatList {
    link: ComponentLink<Self>,
    props: Props,
    audio: Audio,
    _interval: Option<IntervalTask>,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub game: GameView,
    pub profiles: HashMap<u64, Profile>,
    pub user: Option<Profile>,
    pub time_adjustment: i128,
}

pub enum Msg {
    TakeSeat(u32),
    LeaveSeat(u32),
    KickSeat(usize),
    Refresh,
}

impl Component for SeatList {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let interval = props.game.clock.as_ref().map(|_| {
            IntervalService::spawn(Duration::from_millis(250), link.callback(|_| Msg::Refresh))
        });
        SeatList {
            link,
            props,
            audio: Audio::new(),
            _interval: interval,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::TakeSeat(idx) => networking::send(message::GameAction::TakeSeat(idx)),
            Msg::LeaveSeat(idx) => networking::send(message::GameAction::LeaveSeat(idx)),
            Msg::KickSeat(idx) => {
                let player = self.props.game.seats[idx].0;
                if let Some(player) = player {
                    networking::send(message::GameAction::KickPlayer(player));
                }
            }
            Msg::Refresh => {
                let now = js_sys::Date::now() as i128;
                let game = &self.props.game;
                let adj = self.props.time_adjustment;
                if !matches!(game.state, shared::game::GameStateView::Play(_)) {
                    return false;
                }
                let game_clock = match &game.clock {
                    Some(c) => c,
                    None => return false,
                };
                for (idx, clock) in game_clock.clocks.iter().enumerate() {
                    if game.turn != idx as u32 || game_clock.paused {
                        continue;
                    }

                    let time_left = match clock {
                        PlayerClock::Plain {
                            last_time,
                            time_left,
                        } => last_time.0 + time_left.0 + adj - now,
                    };

                    if time_left < 5000 && time_left > 0 {
                        self.audio.play_beep();
                        break;
                    }
                }
            }
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self._interval = props.game.clock.as_ref().map(|_| {
                IntervalService::spawn(
                    Duration::from_millis(250),
                    self.link.callback(|_| Msg::Refresh),
                )
            });
            if props.game.move_number != self.props.game.move_number {
                self.audio.stop_beep();
            }
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

        let now = js_sys::Date::now() as i128;

        let list = game
            .seats
            .iter()
            .enumerate()
            .map(|(idx, (occupant, color, resigned))| {
                let colorname = Color::name(*color);

                let scoretext = match scores {
                    Some(scores) => {
                        format!(" - Score: {:.1}", scores[*color as usize - 1] as f32 / 2.)
                    }
                    None => "".to_owned(),
                };

                let kick = if self.props.user.as_ref().map(|x| x.user_id) == Some(game.owner) {
                    html! {
                        <button onclick=self.link.callback(move |_| Msg::KickSeat(idx))>
                            {"Kick"}
                        </button>
                    }
                } else {
                    html!()
                };

                let resigned_text = if *resigned {
                    " - resigned!"
                } else {
                    ""
                };

                let time_left = if let (false, Some(game_clock)) = (resigned, &game.clock) {
                    let clock = &game_clock.clocks[idx];
                    let adj = self.props.time_adjustment;
                    let time_left = if game.turn == idx as u32 && !game_clock.paused {
                        match clock {
                            PlayerClock::Plain { last_time, time_left } => last_time.0 + time_left.0 + adj - now
                        }
                    } else {
                        match clock {
                            PlayerClock::Plain { time_left, .. } => time_left.0 + adj
                        }
                    };
                    let minutes = time_left / (60 * 1000);
                    let seconds = (time_left / 1000) % 60;
                    Some(if minutes > 0 {
                        format!("- {}min {}s left", minutes, seconds)
                    } else {
                        format!("- {}s left", seconds)
                    })
                } else {
                    None
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
                        kick
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
                        <div class=class style="margin: 5px 0; padding: 0px 5px; padding-top: 5px;">
                            {format!("{}: {} {}{}{}", colorname, nick, scoretext, passed, resigned_text)}
                            {leave}
                            {if_html!(let Some(t) = time_left =>
                                <div style="padding: 10px; font-size: large;">{t}</div>
                            )}
                        </div>
                    }
                } else {
                    html! {
                        <div style="margin: 5px 0;">
                            {format!("{}: unoccupied{}{}", colorname, scoretext, resigned_text)}
                            <button onclick=self.link.callback(move |_| Msg::TakeSeat(idx as _))>
                                {"Take seat"}
                            </button>
                            {if_html!(let Some(t) = time_left =>
                                <div style="padding: 10px; font-size: large;">{t}</div>
                            )}
                        </div>
                    }
                }
            })
            .collect::<Html>();

        html!(<div style="margin: 10px;">{list}</div>)
    }
}
