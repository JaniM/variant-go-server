mod board;
#[path = "../../server/src/game.rs"]
#[allow(dead_code)]
mod game;
mod game_view;
#[path = "../../server/src/message.rs"]
mod message;
mod networking;
mod seats;
mod utils;

use std::collections::HashMap;
use std::time::Duration;
use wasm_bindgen::prelude::*;

use crate::game_view::{GameView, Profile};
use crate::message::{ClientMessage, ServerMessage};
use crate::seats::SeatList;

use yew::prelude::*;
use yew::services::timeout::{TimeoutService, TimeoutTask};

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
    // TODO: Use a proper struct, not magic tuples
    games: Vec<(u32, String)>,
    game: Option<GameView>,
    user: Option<Profile>,
    profiles: HashMap<u64, Profile>,
    debounce_job: Option<TimeoutTask>,
}

enum Msg {
    StartGame,
    ChangeNick(String),
    JoinGame(u32),
    Pass,
    Cancel,
    SetGameStatus(GameView),
    SetOwnProfile(Profile),
    SetProfile(Profile),
    AddGame((u32, String)),
    RemoveGame(u32),
    Render,
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let addgame = link.callback(Msg::AddGame);
        let remove_game = link.callback(Msg::RemoveGame);
        let game = link.callback(Msg::SetGameStatus);
        let set_own_profile = link.callback(Msg::SetOwnProfile);
        let set_profile = link.callback(Msg::SetProfile);
        networking::start_websocket(move |msg| {
            match msg {
                ServerMessage::AnnounceGame { room_id, name } => {
                    addgame.emit((room_id, name));
                }
                ServerMessage::CloseGame { room_id } => {
                    remove_game.emit(room_id);
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
                        room_id,
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
            debounce_job: None,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartGame => networking::send(ClientMessage::StartGame {
                name: self
                    .user
                    .as_ref()
                    .and_then(|p| p.nick.clone())
                    .unwrap_or_else(|| "No nick".to_owned()),
            }),
            Msg::ChangeNick(nick) => networking::send(ClientMessage::Identify {
                token: networking::get_token(),
                nick: Some(nick),
            }),
            Msg::JoinGame(id) => networking::send(ClientMessage::JoinGame(id)),
            Msg::Pass => networking::send(ClientMessage::GameAction(message::GameAction::Pass)),
            Msg::Cancel => networking::send(ClientMessage::GameAction(message::GameAction::Cancel)),
            Msg::SetGameStatus(game) => self.game = Some(game),
            Msg::AddGame(game) => {
                self.games.push(game);
                if self.debounce_job.is_none() {
                    self.debounce_job = Some(TimeoutService::spawn(
                        Duration::from_millis(100),
                        self.link.callback(|_| Msg::Render),
                    ));
                }
                return false;
            }
            Msg::RemoveGame(room_id) => {
                self.games.retain(|g| g.0 != room_id);
                if let Some(game) = &self.game {
                    if game.room_id == room_id {
                        // TODO: show something sensible when a game is closed
                        self.game = None;
                    }
                }
            }
            Msg::SetOwnProfile(profile) => self.user = Some(profile),
            Msg::SetProfile(profile) => {
                self.profiles.insert(profile.user_id, profile);
            }
            Msg::Render => {
                self.debounce_job = None;
                self.games.sort_unstable_by_key(|x| -(x.0 as i32));
            }
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        // TODO: separate out .. everything
        let list = self
            .games
            .iter()
            .map(|&(id, ref name)| {
                html! {
                    <li key={id}>
                        <a href="#" onclick=self.link.callback(move |_| Msg::JoinGame(id))>
                            {format!("{} - {}", id, name)}
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
        let cancel = self.link.callback(|_| Msg::Cancel);

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

            let status = match game.state {
                game::GameState::Play(_) => "Active",
                game::GameState::Scoring(_) => "Scoring",
                game::GameState::Done(_) => "Game over!",
            };

            let pass_button = match game.state {
                game::GameState::Play(_) => html!(<button onclick=pass>{"Pass"}</button>),
                game::GameState::Scoring(_) => html!(<button onclick=pass>{"Accept"}</button>),
                game::GameState::Done(_) => html!(),
            };

            let cancel_button = match game.state {
                game::GameState::Scoring(_) => html!(<button onclick=cancel>{"Cancel"}</button>),
                _ => html!(),
            };

            html!(
                <div>
                    <p>{"Users:"} {userlist}</p>
                    <p>{"Seats"}</p>
                    <SeatList game=game profiles=&self.profiles user=&self.user />
                    <div>{"Status:"} {status} {pass_button} {cancel_button}</div>
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
                <button onclick=self.link.callback(|_| Msg::StartGame)>{ "Start game" }</button>
                {"Games live: "}{self.games.len()}
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
