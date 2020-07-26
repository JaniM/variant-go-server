mod utils;
mod message;
mod networking;

use wasm_bindgen::prelude::*;

use crate::message::{ClientMessage, ServerMessage};


use yew::prelude::*;

struct GameList {
    link: ComponentLink<Self>,
    games: Vec<i32>
}

enum Msg {
    AddGame,
}

impl Component for GameList {
    type Message = Msg;
    type Properties = ();
    fn create(_: Self::Properties, link: ComponentLink<Self>) -> Self {
        GameList {
            link,
            games: vec![]
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::AddGame => networking::send(ClientMessage::StartGame)
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
                <p>{ self.games.iter().map(|g| html!{ <span> {g} </span>}).collect::<Vec<_>>() }</p>
            </div>
        }
    }
}


#[wasm_bindgen(start)]
pub fn run() -> Result<(), JsValue> {
    utils::set_panic_hook();
    let window = web_sys::window().expect("no global `window` exists");
    let document = window.document().expect("should have a document on window");
    let body = document.body().expect("document should have a body");

    let val = document.create_element("p")?;
    val.set_inner_html("Hello from Rust!");

    body.append_child(&val)?;

    networking::start_websocket(|msg| {
        match msg {
            ServerMessage::GameList { games } => {},
            _ => {}
        };
    });

    yew::initialize();
    App::<GameList>::new().mount_to_body();

    Ok(())
}
