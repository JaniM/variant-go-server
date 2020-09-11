use yew::{
    prelude::*,
    services::keyboard::{KeyListenerHandle, KeyboardService},
    services::resize::{ResizeService, ResizeTask, WindowDimensions},
};
use yewtil::NeqAssign;

use std::collections::HashMap;

use crate::{
    agents::game_store,
    board,
    game_view::{GameView, Profile},
    networking,
    seats::SeatList,
};
use game_store::GameStore;
use message::GameAction;
use shared::{game, message};

pub struct GamePane {
    link: ComponentLink<Self>,
    props: Props,
    callbacks: Callbacks,
    game_store: GameStore,
    size: i32,
    _key_listener: KeyListenerHandle,
    _resize_task: ResizeTask,
}

pub enum Msg {
    Pass,
    Cancel,
    GetBoardAt(u32),
    ScanBoard(i32),
    ResetHistory,
    ResizeWindow(WindowDimensions),
    None,
}

#[derive(Properties, Clone, PartialEq)]
pub struct Props {
    pub game: GameView,
    pub user: Option<Profile>,
    pub profiles: HashMap<u64, Profile>,
}

#[derive(Clone)]
struct Callbacks {
    pass: Callback<()>,
    cancel: Callback<()>,
}

impl Component for GamePane {
    type Message = Msg;
    type Properties = Props;

    fn create(props: Self::Properties, link: ComponentLink<Self>) -> Self {
        let callbacks = Callbacks {
            pass: link.callback(|_| Msg::Pass),
            cancel: link.callback(|_| Msg::Cancel),
        };

        // Currently the state is passed back through props so we don't care about the output
        let game_store = GameStore::bridge(Callback::from(|_| ()));

        let key_listener = KeyboardService::register_key_down(
            &yew::utils::document(),
            link.callback(|event: KeyboardEvent| match event.key().as_str() {
                "ArrowRight" => Msg::ScanBoard(1),
                "ArrowLeft" => Msg::ScanBoard(-1),
                _ => Msg::None,
            }),
        );

        let resize_task = ResizeService::new().register(link.callback(Msg::ResizeWindow));
        let dimensions =
            WindowDimensions::get_dimensions(&web_sys::window().expect("window not found"));
        let size = size_from_dimensions(dimensions);

        GamePane {
            link,
            props,
            callbacks,
            game_store,
            size,
            _key_listener: key_listener,
            _resize_task: resize_task,
        }
    }

    fn update(&mut self, msg: Self::Message) -> ShouldRender {
        match msg {
            Msg::Pass => networking::send(GameAction::Pass),
            Msg::Cancel => networking::send(GameAction::Cancel),
            Msg::GetBoardAt(turn) => {
                self.game_store.get_board_at(turn);
            }
            Msg::ScanBoard(diff) => {
                self.game_store.scan_board(diff);
            }
            Msg::ResetHistory => {
                self.game_store.set_game_history(None);
            }
            Msg::ResizeWindow(dimensions) => {
                self.size = size_from_dimensions(dimensions);
                return true;
            }
            Msg::None => {}
        }
        false
    }

    fn change(&mut self, props: Self::Properties) -> ShouldRender {
        self.props.neq_assign(props)
    }

    fn view(&self) -> Html {
        let Props {
            user,
            game,
            profiles,
        } = &self.props;
        let Callbacks { pass, cancel } = &self.callbacks;

        // FIXME: Reforming the callbacks prevents yew from optimizing for equality.
        // Either patch it upstream or make the callbacks have the proper shape.
        let pass = pass.reform(|_| ());
        let cancel = cancel.reform(|_| ());

        let userlist = game
            .members
            .iter()
            .map(|id| {
                let nick = profiles
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

        let game_done = matches!(game.state, game::GameState::Done(_));

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
            game::GameState::FreePlacement(_) => html!(<button onclick=cancel>{"Clear"}</button>),
            game::GameState::Play(_) => html!(<button onclick=cancel>{"Undo"}</button>),
            game::GameState::Scoring(_) => html!(<button onclick=cancel>{"Cancel"}</button>),
            _ => html!(),
        };

        let game_length = game.move_number;
        let view_turn = match &game.history {
            Some(h) => h.move_number,
            None => game.move_number,
        };

        let turn_bar_buttons = if !game.mods.no_history || game_done {
            html! {
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
                        onclick=self.link.callback(|_| Msg::ResetHistory)
                        disabled={view_turn >= game.move_number} >
                        {">>>"}
                    </button>
                </div>
            }
        } else {
            html!()
        };

        let turn_bar = html! {
            <div style="display: flex;">
                <div style="width: 200px;">
                <span>{"Turn "}{view_turn}{"/"}{game.move_number}</span>
                <span>{if game.history.is_some() { "(history)" } else { "" }}</span>
                </div>
                {turn_bar_buttons}
            </div>
        };

        let game_container_style = format!("width: {}px; margin: auto 0;", self.size);

        html!(
            <>
            <div style="flex-grow: 1; margin: 10px; display: flex; justify-content: center;">
                <div style=game_container_style>
                    <div>{"Status:"} {status} {pass_button} {cancel_button} {hidden_stones_left}</div>
                    <board::Board game=game size=self.size/>
                    {turn_bar}
                </div>
            </div>
            <div style="width: 300px; overflow: hidden; border-left: 2px solid #dedede; padding: 10px; padding-left: 10px;">
                <div><a href="https://github.com/JaniM/variant-go-server" target="_blank">{"Github"}</a>{" / "}<a href="https://discord.gg/qzqwEV4" target="_blank">{"Discord"}</a></div>
                <div>{"Seats"}</div>
                <SeatList game=game profiles=profiles user=user />
                {"Users"}
                <div>{userlist}</div>
            </div>
            </>
        )
    }
}

fn size_from_dimensions(dimensions: WindowDimensions) -> i32 {
    // FIXME: This is naive and lazy but works well enough.
    // The buffer is used for vertically surrounding elements.
    let buffer = 200;
    let mut size = i32::min(dimensions.width - 300, dimensions.height) - buffer;
    if size < 200 {
        size = 200;
    }
    size
}
