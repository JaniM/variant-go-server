use yew::agent::{Agent, AgentLink};
use yew::prelude::*;

use std::collections::HashMap;

use store::{store, Bridgeable, Store, StoreBridge, StoreWrapper};

#[derive(Clone, Debug)]
pub struct BoardState {
    pub board_displacement: (i32, i32),
    pub toroidal_edge_size: i32,
}

store! {
    store BoardStore,
    state BoardStoreState,
    request Request {
        set_board_state => SetState(room_id: u32, state: BoardState),
        refresh => Refresh,
    }
}

#[derive(Debug)]
pub enum Action {
    SetState(u32, BoardState),
    Refresh,
}

pub struct BoardStoreState {
    pub boards: HashMap<u32, BoardState>,
}

impl Store for BoardStoreState {
    type Action = Action;
    type Input = Request;

    fn new() -> Self {
        BoardStoreState {
            boards: HashMap::new(),
        }
    }

    fn handle_input(&self, link: AgentLink<StoreWrapper<Self>>, msg: Self::Input) {
        match msg {
            Request::SetState(room_id, state) => {
                link.send_message(Action::SetState(room_id, state));
            }
            Request::Refresh => {
                link.send_message(Action::Refresh);
            }
        }
    }

    fn reduce(&mut self, msg: Self::Action) {
        match msg {
            Action::SetState(room_id, state) => {
                self.boards.insert(room_id, state);
            }
            Action::Refresh => {}
        }
    }
}
