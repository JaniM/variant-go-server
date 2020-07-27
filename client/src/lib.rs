mod utils;
#[path = "../../server/src/message.rs"]
mod message;
mod networking;
mod board;

use wasm_bindgen::prelude::*;

use crate::message::{ClientMessage, ServerMessage};


use yew::prelude::*;

struct GameList {
    link: ComponentLink<Self>,
    games: Vec<u32>
}

enum Msg {
    AddGame,
    SetGameList(Vec<u32>)
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        let gamelist = link.callback(|v| Msg::SetGameList(v));
        networking::start_websocket(move |msg| {
            match msg {
                ServerMessage::GameList { games } => {
                    gamelist.emit(games);
                },
                _ => {}
            };
        });

        GameList {
            link,
            games: vec![]
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AddGame => networking::send(ClientMessage::StartGame),
            Msg::SetGameList(games) => self.games = games
        }
        true
    }

    fn change(&mut self, _: Self::Properties) -> ShouldRender {
        false
    }

    fn view(&self) -> Html {
        html! {
            <div>
                <button onclick=self.link.callback(|_| Msg::AddGame)>{ "+1" }</button>
                <ul>{ self.games.iter().map(|g| html!{ <li> {g} </li>}).collect::<Html>() }</ul>
                <div>
                    <board::Board />
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
