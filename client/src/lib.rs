#![recursion_limit = "1024"]

mod agents;
mod board;
mod create_game;
mod game_view;
mod networking;
mod seats;
mod text_input;
mod utils;

use std::collections::HashMap;
use std::time::Duration;
use wasm_bindgen::prelude::*;

use crate::agents::game_store;
use crate::create_game::CreateGameView;
use crate::game_view::{GameView, Profile};
use crate::seats::SeatList;
use crate::text_input::TextInput;

use shared::game;
use shared::message::{self, ClientMessage, ServerMessage};

use yew::prelude::*;
use yew::services::keyboard::{KeyListenerHandle, KeyboardService};
use yew::services::timeout::{TimeoutService, TimeoutTask};

use store::ReadOnly;

use itertools::Itertools;

enum Pane {
    CreateGame,
    Board,
}

#[derive(Copy, Clone, PartialEq, Debug)]
enum Theme {
    White,
    Dark,
}

impl std::fmt::Display for Theme {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Theme {
    fn get() -> Theme {
        let val = utils::local_storage().get_item("theme").unwrap();
        match val.as_ref().map(|x| &**x) {
            Some("White") => Theme::White,
            Some("Dark") => Theme::Dark,
            _ => Theme::White,
        }
    }

    fn save(&self) {
        utils::local_storage()
            .set_item("theme", &format!("{:?}", self))
            .unwrap();
    }
}

struct GameList {
    link: ComponentLink<Self>,
    // TODO: Use a proper struct, not magic tuples
    games: Vec<(u32, String)>,
    game: Option<GameView>,
    user: Option<Profile>,
    profiles: HashMap<u64, Profile>,
    pane: Pane,
    debounce_job: Option<TimeoutTask>,
    theme: Theme,
    error: Option<(message::Error, TimeoutTask)>,
    #[allow(dead_code)]
    key_listener: KeyListenerHandle,
    game_store: game_store::GameStore,
}

enum Msg {
    StartGame,
    ChangeNick(String),
    JoinGame(u32),
    Pass,
    Cancel,
    SetGameStatus(GameView),
    GameStoreMsg(ReadOnly<game_store::GameStoreState>),
    SetGameHistory(Option<game::GameHistory>),
    SetOwnProfile(Profile),
    SetProfile(Profile),
    AddGame((u32, String)),
    RemoveGame(u32),
    SetPane(Pane),
    SetTheme(Theme),
    SetError(Option<message::Error>),
    GetBoardAt(u32),
    ScanBoard(i32),
    Render,
    None,
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let addgame = link.callback(Msg::AddGame);
        let remove_game = link.callback(Msg::RemoveGame);
        let game = link.callback(Msg::SetGameStatus);
        let set_game_history = link.callback(Msg::SetGameHistory);
        let set_own_profile = link.callback(Msg::SetOwnProfile);
        let set_profile = link.callback(Msg::SetProfile);
        let set_error = link.callback(Msg::SetError);
        networking::start_websocket(move |msg| {
            match msg {
                Ok(ServerMessage::AnnounceGame { room_id, name }) => {
                    addgame.emit((room_id, name));
                }
                Ok(ServerMessage::CloseGame { room_id }) => {
                    remove_game.emit(room_id);
                }
                Ok(ServerMessage::GameStatus {
                    room_id,
                    members,
                    seats,
                    board,
                    board_visibility,
                    hidden_stones_left,
                    turn,
                    state,
                    size,
                    mods,
                    points,
                    move_number,
                }) => {
                    game.emit(GameView {
                        room_id,
                        members,
                        seats,
                        board,
                        board_visibility,
                        hidden_stones_left,
                        turn,
                        state,
                        size,
                        mods,
                        points,
                        move_number,
                        history: None,
                    });
                }
                Ok(ServerMessage::BoardAt(view)) => {
                    set_game_history.emit(Some(view));
                }
                Ok(ServerMessage::Identify {
                    user_id,
                    token,
                    nick,
                }) => {
                    networking::set_token(&token);
                    set_own_profile.emit(Profile { user_id, nick });
                }
                Ok(ServerMessage::Profile(message::Profile { user_id, nick })) => {
                    set_profile.emit(Profile { user_id, nick });
                }
                Ok(ServerMessage::Error(err)) => {
                    set_error.emit(Some(err));
                }
                Err(networking::ServerError::LostConnection) => {
                    set_error.emit(Some(message::Error::other(
                        "Lost connection, reconnecting...",
                    )));
                }
                Err(networking::ServerError::Clear) => {
                    set_error.emit(None);
                }
                _ => {}
            };
        })
        .unwrap();

        let key_listener = KeyboardService::register_key_down(
            &yew::utils::document(),
            link.callback(|event: web_sys::KeyboardEvent| match event.key().as_str() {
                "ArrowRight" => Msg::ScanBoard(1),
                "ArrowLeft" => Msg::ScanBoard(-1),
                _ => Msg::None,
            }),
        );

        let game_store = game_store::GameStore::bridge(link.callback(Msg::GameStoreMsg));

        GameList {
            link,
            games: vec![],
            game: None,
            user: None,
            profiles: HashMap::new(),
            pane: Pane::Board,
            debounce_job: None,
            theme: Theme::get(),
            error: None,
            key_listener,
            game_store,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartGame => self.pane = Pane::CreateGame,
            Msg::ChangeNick(nick) => networking::send(ClientMessage::Identify {
                token: networking::get_token(),
                nick: Some(nick),
            }),
            Msg::JoinGame(id) => {
                networking::send(ClientMessage::JoinGame(id));
                self.pane = Pane::Board;
            }
            Msg::Pass => networking::send(ClientMessage::GameAction(message::GameAction::Pass)),
            Msg::Cancel => networking::send(ClientMessage::GameAction(message::GameAction::Cancel)),
            Msg::SetGameStatus(game) => {
                self.game_store.set_game(game);
                return false;
            }
            Msg::GameStoreMsg(store) => {
                let store = store.borrow();
                self.game = store.game.clone();
            }
            Msg::SetGameHistory(view) => {
                self.game_store.set_game_history(view);
                return false;
            }
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
            Msg::SetOwnProfile(profile) => {
                self.profiles.insert(profile.user_id, profile.clone());
                self.user = Some(profile);
            }
            Msg::SetProfile(profile) => {
                self.profiles.insert(profile.user_id, profile);
            }
            Msg::SetPane(pane) => self.pane = pane,
            Msg::SetTheme(theme) => {
                self.theme = theme;
                self.theme.save();
            }
            Msg::SetError(err) => {
                self.error = err.map(|err| {
                    (
                        err,
                        TimeoutService::spawn(
                            Duration::from_secs(10),
                            self.link.callback(|_| Msg::SetError(None)),
                        ),
                    )
                });
            }
            Msg::GetBoardAt(turn) => {
                self.game_store.get_board_at(turn);
            }
            Msg::ScanBoard(diff) => {
                self.game_store.scan_board(diff);
            }
            Msg::Render => {
                self.debounce_job = None;
                self.games.sort_unstable_by_key(|x| -(x.0 as i32));
                self.games = self
                    .games
                    .iter()
                    .cloned()
                    .dedup_by(|x, y| x.0 == y.0)
                    .collect();
            }
            Msg::None => {
                return false;
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
                        <a href=format!("#{}", id) onclick=self.link.callback(move |_| Msg::JoinGame(id))>
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
                        <>
                        <span style="padding: 0px 10px">
                            {format!("{}", nick)}
                        </span>
                        <br />
                        </>
                    )
                })
                .collect::<Html>();

            let status = match game.state {
                game::GameState::FreePlacement(_) => "Free placement",
                game::GameState::Play(_) => "Active",
                game::GameState::Scoring(_) => "Scoring",
                game::GameState::Done(_) => "Game over!",
            };

            let hidden_stones_left = if game.hidden_stones_left > 0 {
                html!(<>{"Opponents' hidden stones left: "}{game.hidden_stones_left}</>)
            } else {
                html!()
            };

            let pass_button = match game.state {
                game::GameState::FreePlacement(_) => html!(<button onclick=pass>{"Ready"}</button>),
                game::GameState::Play(_) => html!(<button onclick=pass>{"Pass"}</button>),
                game::GameState::Scoring(_) => html!(<button onclick=pass>{"Accept"}</button>),
                game::GameState::Done(_) => html!(),
            };

            let cancel_button = match game.state {
                game::GameState::FreePlacement(_) => {
                    html!(<button onclick=cancel>{"Clear"}</button>)
                }
                game::GameState::Play(_) => html!(<button onclick=cancel>{"Undo"}</button>),
                game::GameState::Scoring(_) => html!(<button onclick=cancel>{"Cancel"}</button>),
                _ => html!(),
            };

            let game_length = game.move_number;
            let view_turn = match &game.history {
                Some(h) => h.move_number,
                None => game.move_number,
            };

            let turn_bar = html! {
                <div style="display: flex;">
                    <div style="width: 200px;">
                    <span>{"Turn "}{view_turn}{"/"}{game.move_number}</span>
                    <span>{if game.history.is_some() { "(history)" } else { "" }}</span>
                    </div>
                    <div style="flex-grow: 1; display: flex; justify-content: center; margin-left: -200px;">
                    <button
                        onclick=self.link.callback(move |_| Msg::GetBoardAt(0))
                        disabled={view_turn == 0} >
                        {"<<<"}
                    </button>
                    <button
                        onclick=self.link.callback(move |_|
                            Msg::GetBoardAt(view_turn.saturating_sub(5)))
                        disabled={view_turn == 0} >
                        {"<<"}
                    </button>
                    <button
                        onclick=self.link.callback(move |_| Msg::GetBoardAt(view_turn-1))
                        disabled={view_turn == 0} >
                        {"<"}
                    </button>
                    <button
                        onclick=self.link.callback(move |_| Msg::GetBoardAt(view_turn+1))
                        disabled={view_turn >= game.move_number} >
                        {">"}
                    </button>
                    <button
                        onclick=self.link.callback(move |_|
                            Msg::GetBoardAt((view_turn+5).min(game_length)))
                        disabled={view_turn >= game.move_number} >
                        {">>"}
                    </button>
                    <button
                        onclick=self.link.callback(|_| Msg::SetGameHistory(None))
                        disabled={view_turn >= game.move_number} >
                        {">>>"}
                    </button>
                    </div>
                </div>
            };

            html!(
                <>
                <div style="flex-grow: 1; margin: 10px; display: flex; justify-content: center;">
                    <div style="width: 800px; margin: auto 0;">
                        <div>{"Status:"} {status} {pass_button} {cancel_button} {hidden_stones_left}</div>
                        <board::Board game=game/>
                        {turn_bar}
                    </div>
                </div>
                <div style="width: 300px; overflow: hidden; border-left: 2px solid #dedede; padding: 10px; padding-left: 10px;">
                    <div><a href="https://github.com/JaniM/variant-go-server" target="_blank">{"Github"}</a>{" / "}<a href="https://discord.gg/qzqwEV4" target="_blank">{"Discord"}</a></div>
                    <div>{"Seats"}</div>
                    <SeatList game=game profiles=&self.profiles user=&self.user />
                    {"Users"}
                    <div>{userlist}</div>
                </div>
                </>
            )
        } else {
            html!(<p>{"Join a game!"}</p>)
        };

        let right_panel = match self.pane {
            Pane::Board => gameview,
            Pane::CreateGame => html!(<CreateGameView
                user=self.user.as_ref().unwrap()
                oncreate=self.link.callback(|_| Msg::SetPane(Pane::Board)) />),
        };

        let select_theme = self.link.callback(|event| match event {
            ChangeData::Select(elem) => {
                let value = elem.selected_index();
                Msg::SetTheme(match value {
                    0 => Theme::White,
                    1 => Theme::Dark,
                    _ => unreachable!(),
                })
            }
            _ => unreachable!(),
        });

        let theme_selection = html! {
            <select
                onchange=select_theme
            >
                <option value=Theme::White selected=self.theme == Theme::White>{ "White" }</option>
                <option value=Theme::Dark selected=self.theme == Theme::Dark>{ "Dark" }</option>
            </select>
        };

        let class = match self.theme {
            Theme::White => "",
            Theme::Dark => "dark",
        };

        let error_box = if let Some((error, _)) = &self.error {
            let text = match error {
                message::Error::GameStartTimer(x) => {
                    format!("You can only create a game every 2 minutes ({}s left)", x)
                }
                message::Error::Other(x) => x.to_string(),
            };
            html! {
                <div
                    class=("error-box", class)
                    onclick=self.link.callback(|_| Msg::SetError(None))>
                    {text}
                </div>
            }
        } else {
            html!()
        };

        html! {
        <div
            id="main"
            class=class
            style="display: flex; flex-direction: row; min-height: 100vh;">
            <div style="width: 300px; border-right: 2px solid #dedede; padding: 10px; margin-right: 10px;">
                <div style="width: 100%; margin-bottom: 10px;">
                    <button
                        style="width: 100%;"
                        onclick=self.link.callback(|_| Msg::StartGame)>
                        { "Start game" }
                    </button>
                </div>
                <div>{"Theme: "}{theme_selection}</div>
                <div>
                    {"Nick: "}
                    <TextInput value=nick onsubmit=nick_enter />
                </div>
                {"Games live: "}{self.games.len()}
                <ul>
                    {list}
                </ul>
            </div>
            {right_panel}
            {error_box}
        </div>
        }
    }
}

/// This runs.
#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();

    yew::initialize();
    App::<GameList>::new().mount_to_body();

    Ok(())
}
