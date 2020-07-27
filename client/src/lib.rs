mod utils;
#[path = "../../server/src/message.rs"]
mod message;
mod networking;
mod board;
mod game_view;

use wasm_bindgen::prelude::*;

use crate::message::{ClientMessage, ServerMessage};
use crate::game_view::GameView;

use yew::prelude::*;

struct GameList {
    link: ComponentLink<Self>,
    games: Vec<u32>,
    game: Option<GameView>
}

enum Msg {
    AddGame,
    JoinGame(u32),
    SetGameStatus(GameView),
    SetGameList(Vec<u32>)
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let gamelist = link.callback(Msg::SetGameList);
        let game = link.callback(Msg::SetGameStatus);
        networking::start_websocket(move |msg| {
            match msg {
                ServerMessage::GameList { games } => {
                    gamelist.emit(games);
                },
                ServerMessage::GameStatus { room_id, moves } => {
                    game.emit(GameView { moves });
                }
                _ => {}
            };
        }).unwrap();

        GameList {
            link,
            games: vec![],
            game: None
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AddGame => networking::send(ClientMessage::StartGame),
            Msg::JoinGame(id) => networking::send(ClientMessage::JoinGame(id)),
            Msg::SetGameStatus(game) => self.game = Some(game),
            Msg::SetGameList(games) => self.games = games
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        let list = self.games.iter().map(|&g| html!{
            <li onclick=self.link.callback(move |_| Msg::JoinGame(g))>
                {g}
            </li>
        }).collect::<Html>();
        html! {
            <div>
                <button onclick=self.link.callback(|_| Msg::AddGame)>{ "+1" }</button>
                <ul>
                    {list}
                </ul>
                <div>
                {
                    if let Some(game) = &self.game {
                        html!(<board::Board game=game/>)
                    } else {
                        html!(<p>{"Join a game!"}</p>)
                    }
                }
                </div>
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
