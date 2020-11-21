#![recursion_limit = "1024"]

mod agents;
mod board;
mod create_game;
mod game_pane;
mod game_view;
mod networking;
mod seats;
mod text_input;
#[macro_use]
mod utils;
mod palette;

use std::collections::HashMap;
use std::time::Duration;
use wasm_bindgen::prelude::*;

use crate::agents::game_store;
use crate::create_game::CreateGameView;
use crate::game_pane::GamePane;
use crate::game_view::{GameView, Profile};
use crate::palette::PaletteOption;
use crate::text_input::TextInput;

use shared::game;
use shared::message::{self, ClientMessage, ServerMessage};

use yew::prelude::*;
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
        match val.as_deref() {
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

struct GameApp {
    link: ComponentLink<Self>,
    // TODO: Use a proper struct, not magic tuples
    games: Vec<(u32, String)>,
    game: Option<GameView>,
    user: Option<Profile>,
    profiles: HashMap<u64, Profile>,
    pane: Pane,
    debounce_job: Option<TimeoutTask>,
    theme: Theme,
    palette: PaletteOption,
    error: Option<(message::Error, TimeoutTask)>,
    #[allow(dead_code)]
    game_store: game_store::GameStore,
}

#[allow(clippy::large_enum_variant)]
enum Msg {
    StartGame,
    ChangeNick(String),
    JoinGame(u32),
    SetGameStatus(GameView),
    GameStoreEvent(ReadOnly<game_store::GameStoreState>),
    SetGameHistory(Option<game::GameHistory>),
    SetOwnProfile(Profile),
    SetProfile(Profile),
    AddGame((u32, String)),
    RemoveGame(u32),
    SetPane(Pane),
    SetTheme(Theme),
    SetPalette(PaletteOption),
    SetError(Option<message::Error>),
    Render,
}

impl Component for GameApp {
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
                    owner,
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
                    clock,
                }) => {
                    game.emit(GameView {
                        room_id,
                        owner,
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
                        clock,
                    });
                }
                Ok(ServerMessage::BoardAt { view, .. }) => {
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
                Ok(ServerMessage::SGF { sgf, room_id }) => {
                    web_sys::console::log_1(&JsValue::from_str(&sgf));
                    let res = utils::download_file(&format!("{}.sgf", room_id), &sgf);
                    if let Err(e) = res {
                        web_sys::console::log_1(&e);
                    }
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

        let game_store = game_store::GameStore::bridge(link.callback(Msg::GameStoreEvent));

        let hash = utils::get_hash();
        let game_loaded = hash.starts_with('#') && hash[1..].parse::<u32>().is_ok();

        GameApp {
            link,
            games: vec![],
            game: None,
            user: None,
            profiles: HashMap::new(),
            pane: if game_loaded {
                Pane::Board
            } else {
                Pane::CreateGame
            },
            debounce_job: None,
            theme: Theme::get(),
            palette: PaletteOption::get(),
            error: None,
            game_store,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::StartGame => {
                self.pane = Pane::CreateGame;
                true
            }
            Msg::ChangeNick(nick) => {
                networking::send(ClientMessage::Identify {
                    token: networking::get_token(),
                    nick: Some(nick),
                });
                false
            }
            Msg::JoinGame(id) => {
                networking::send(ClientMessage::JoinGame(id));
                self.pane = Pane::Board;
                true
            }
            Msg::SetGameStatus(game) => {
                self.game_store.set_game(game);
                false
            }
            Msg::GameStoreEvent(store) => {
                let store = store.borrow();
                self.game = store.game.clone();
                true
            }
            Msg::SetGameHistory(view) => {
                self.game_store.set_game_history(view);
                false
            }
            Msg::AddGame(game) => {
                self.games.push(game);
                if self.debounce_job.is_none() {
                    self.debounce_job = Some(TimeoutService::spawn(
                        Duration::from_millis(100),
                        self.link.callback(|_| Msg::Render),
                    ));
                }
                false
            }
            Msg::RemoveGame(room_id) => {
                self.games.retain(|g| g.0 != room_id);
                if let Some(game) = &self.game {
                    if game.room_id == room_id {
                        // TODO: show something sensible when a game is closed
                        self.game = None;
                    }
                }
                true
            }
            Msg::SetOwnProfile(profile) => {
                self.profiles.insert(profile.user_id, profile.clone());
                self.user = Some(profile);
                true
            }
            Msg::SetProfile(profile) => {
                self.profiles.insert(profile.user_id, profile);
                true
            }
            Msg::SetPane(pane) => {
                self.pane = pane;
                true
            }
            Msg::SetTheme(theme) => {
                self.theme = theme;
                self.theme.save();
                true
            }
            Msg::SetPalette(palette) => {
                self.palette = palette;
                self.palette.save();
                true
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
                true
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
                true
            }
        }
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

        let gameview = if let Some(game) = &self.game {
            html!(
                <GamePane
                    user=&self.user
                    profiles=&self.profiles
                    palette=&self.palette
                    game=game />
            )
        } else {
            html!(<p>{"Join a game!"}</p>)
        };

        let right_panel = match self.pane {
            Pane::Board => gameview,
            Pane::CreateGame if self.user.is_some() => html! {
                <>
                    <CreateGameView
                        user=self.user.as_ref().unwrap()
                        oncreate=self.link.callback(|_| Msg::SetPane(Pane::Board)) />
                    <div style="width: 300px; overflow: hidden; border-left: 2px solid #dedede; padding: 10px; padding-left: 10px;">
                        <div>
                            <a href="https://github.com/JaniM/variant-go-server" target="_blank">{"Github"}</a>
                            {" / "}
                            <a href="https://discord.gg/qzqwEV4" target="_blank">{"Discord"}</a>
                            {" / "}
                            <a href="https://www.patreon.com/variantgo" target="_blank">{"Support"}</a>
                            {" / "}
                            <a href="https://github.com/JaniM/variant-go-server/blob/master/privacy_policy.md" target="_blank">{"Privacy policy"}</a>
                        </div>
                    </div>
                </>
            },
            _ => html!(),
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

        let select_palette = self.link.callback(|event| match event {
            ChangeData::Select(elem) => {
                let value = elem.selected_index();
                Msg::SetPalette(match value {
                    0 => PaletteOption::Normal,
                    1 => PaletteOption::Colorblind,
                    _ => unreachable!(),
                })
            }
            _ => unreachable!(),
        });

        let palette_selection = html! {
            <select
                onchange=select_palette
            >
                <option value=PaletteOption::Normal selected=self.palette == PaletteOption::Normal>{ "Normal" }</option>
                <option value=PaletteOption::Colorblind selected=self.palette == PaletteOption::Colorblind>{ "Colorblind" }</option>
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
                message::Error::Game { error, .. } => format!("{:?}", error),
                message::Error::RateLimit => "You're too fast!".to_string(),
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
            style="display: flex; flex-direction: row; height: 100%;">
            <div style="width: 300px; border-right: 2px solid #dedede; padding: 10px; margin-right: 10px; max-height: 100%; overflow-y: auto;">
                <div style="width: 100%; margin-bottom: 10px;">
                    <button
                        style="width: 100%;"
                        onclick=self.link.callback(|_| Msg::StartGame)>
                        { "Create game" }
                    </button>
                </div>
                <div>{"Theme: "}{theme_selection}{" Board: "}{palette_selection}</div>
                <div>
                    {"Nickname: "}
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
    App::<GameApp>::new().mount_to_body();

    Ok(())
}
