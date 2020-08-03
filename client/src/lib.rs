mod board;
mod game_view;
#[path = "../../server/src/message.rs"]
mod message;
#[path = "../../server/src/game.rs"]
#[allow(dead_code)]
mod game;
mod networking;
mod utils;

use wasm_bindgen::prelude::*;

use crate::game_view::{GameView, Profile};
use crate::message::{ClientMessage, ServerMessage};
use std::collections::HashMap;

use yew::prelude::*;

struct TextInput {
    link: ComponentLink<Self>,
    text: String,
    props: TextInputProperties,
}

enum TextInputMsg {
    SetText(String),
    Submit,
    None,
}

#[derive(Properties, Clone, PartialEq)]
struct TextInputProperties {
    value: String,
    onsubmit: Callback<String>,
}

impl Component for TextInput {
    type Message = TextInputMsg;
    type Properties = TextInputProperties;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        TextInput {
            link,
            text: props.value.clone(),
            props,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            TextInputMsg::SetText(text) => self.text = text,
            TextInputMsg::Submit => self.props.onsubmit.emit(self.text.clone()),
            TextInputMsg::None => return false,
        }
        true
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        if self.props != props {
            self.props = props;
            self.text = self.props.value.clone();
            true
        } else {
            false
        }
    }

    fn view(&self) -> Html {
        html! {
            <input
                type="text"
                value=&self.text
                oninput=self.link.callback(|e: InputData| TextInputMsg::SetText(e.value))
                onkeypress=self.link.callback(move |e: KeyboardEvent| {
                    if e.key() == "Enter" { TextInputMsg::Submit } else { TextInputMsg::None }
                })
                />
        }
    }
}

struct GameList {
    link: ComponentLink<Self>,
    games: Vec<u32>,
    game: Option<GameView>,
    user: Option<Profile>,
    profiles: HashMap<u64, Profile>,
}

enum Msg {
    AddGame,
    ChangeNick(String),
    TakeSeat(u32),
    LeaveSeat(u32),
    JoinGame(u32),
    Pass,
    SetGameStatus(GameView),
    SetOwnProfile(Profile),
    SetProfile(Profile),
    SetGameList(Vec<u32>),
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let gamelist = link.callback(Msg::SetGameList);
        let game = link.callback(Msg::SetGameStatus);
        let set_own_profile = link.callback(Msg::SetOwnProfile);
        let set_profile = link.callback(Msg::SetProfile);
        networking::start_websocket(move |msg| {
            match msg {
                ServerMessage::GameList { games } => {
                    gamelist.emit(games);
                }
                ServerMessage::GameStatus {
                    room_id,
                    members,
                    seats,
                    board,
                    turn,
                    state,
                } => {
                    game.emit(GameView {
                        members,
                        seats,
                        board,
                        turn,
                        state,
                    });
                }
                ServerMessage::Identify {
                    user_id,
                    token,
                    nick,
                } => {
                    networking::set_token(&token);
                    set_own_profile.emit(Profile { user_id, nick });
                }
                ServerMessage::Profile(message::Profile { user_id, nick }) => {
                    set_profile.emit(Profile { user_id, nick });
                }
                _ => {}
            };
        })
        .unwrap();

        GameList {
            link,
            games: vec![],
            game: None,
            user: None,
            profiles: HashMap::new(),
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AddGame => networking::send(ClientMessage::StartGame),
            Msg::ChangeNick(nick) => networking::send(ClientMessage::Identify {
                token: networking::get_token(),
                nick: Some(nick),
            }),
            Msg::TakeSeat(idx) => networking::send(ClientMessage::GameAction(
                message::GameAction::TakeSeat(idx),
            )),
            Msg::LeaveSeat(idx) => networking::send(ClientMessage::GameAction(
                message::GameAction::LeaveSeat(idx),
            )),
            Msg::JoinGame(id) => networking::send(ClientMessage::JoinGame(id)),
            Msg::Pass => networking::send(ClientMessage::GameAction(
                message::GameAction::Pass)),
            Msg::SetGameStatus(game) => self.game = Some(game),
            Msg::SetGameList(games) => self.games = games,
            Msg::SetOwnProfile(profile) => self.user = Some(profile),
            Msg::SetProfile(profile) => {
                self.profiles.insert(profile.user_id, profile);
            }
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let list = self
            .games
            .iter()
            .map(|&g| {
                html! {
                    <li>
                        <a href="#" onclick=self.link.callback(move |_| Msg::JoinGame(g))>
                            {g}
                        </a>
                    </li>
                }
            })
            .collect::<Html>();
        let nick = self
            .user
            .as_ref()
            .and_then(|x| x.nick.as_ref())
            .map(|x| &**x)
            .unwrap_or("");
        let nick_enter = self.link.callback(Msg::ChangeNick);
        let pass = self.link.callback(|_| Msg::Pass);

        let gameview = if let Some(game) = &self.game {
            let userlist = game
                .members
                .iter()
                .map(|id| {
                    let nick = self
                        .profiles
                        .get(id)
                        .and_then(|p| p.nick.as_ref())
                        .map(|n| &**n)
                        .unwrap_or("no nick");
                    html!(
                        <span style="padding: 0px 10px">
                            {format!("{} ({})", id, nick)}
                        </span>
                    )
                })
                .collect::<Html>();

            let seats = game.seats.iter().enumerate()
                .map(|(idx, (occupant, color))| {
                    let colorname = match color {
                        1 => "Black",
                        2 => "White",
                        _ => "???",
                    };

                    if let Some(id) = occupant {
                        let nick = self.profiles.get(id)
                            .and_then(|p| p.nick.as_ref())
                            .map(|n| &**n)
                            .unwrap_or("no nick");
                        let leave = if self.user.as_ref().map(|x| x.user_id) == Some(*id) {
                            html!(<button onclick=self.link.callback(move |_| Msg::LeaveSeat(idx as _))>
                                {"Leave seat"}
                            </button>)
                        } else {
                            html!()
                        };

                        let style = if game.turn == idx as u32 { "background-color: #eeeeee;" } else { "" };

                        html!{
                            <li style=style>
                                {format!("{}: {} ({})", colorname, id, nick)}
                                {leave}
                            </li>
                        }
                    } else {
                        html!{
                            <li>
                                {format!("{}: unoccupied", colorname)}
                                <button onclick=self.link.callback(move |_| Msg::TakeSeat(idx as _))>
                                    {"Take seat"}
                                </button>
                            </li>
                        }
                    }
                })
                .collect::<Html>();

            let status = match game.state {
                game::GameState::Play => "Active",
                game::GameState::Scoring(_) => "Scoring",
                game::GameState::Done => "Game over!",
            };

            html!(
                <div>
                    <p>{"Users:"} {userlist}</p>
                    <p>{"Seats"}</p>
                    <ul>{seats}</ul>
                    <div>{"Status:"} {status} <button onclick=pass>{"Pass"}</button></div>
                    <board::Board game=game/>
                </div>
            )
        } else {
            html!(<p>{"Join a game!"}</p>)
        };

        html! {
            <div style="display: flex; flex-direction: row; min-height: 100vh;">
                <div style="min-width: 300px; border-right: 2px solid black; margin: 10px;">
                    <div>
                        {"Nick:"}
                        <TextInput value=nick onsubmit=nick_enter />
                    </div>
                    <button onclick=self.link.callback(|_| Msg::AddGame)>{ "+1" }</button>
                    <ul>
                        {list}
                    </ul>
                </div>
                <div> {gameview} </div>
            </div>
        }
    }
}

#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();

    yew::initialize();
    App::<GameList>::new().mount_to_body();

    Ok(())
}
