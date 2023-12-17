use std::collections::HashMap;

use crate::networking::use_websocket_provider;
use dioxus::prelude::*;
use dioxus_signals::Signal;
use gloo_storage::Storage as _;
use shared::message::{ClientMessage, Profile, ServerMessage};

#[derive(Clone, Copy)]
pub(crate) struct ClientState {
    pub(crate) user: Signal<Profile>,
    pub(crate) profiles: Signal<HashMap<u64, Profile>>,
}

impl ClientState {
    fn new() -> Self {
        Self {
            user: Signal::new(Profile::default()),
            profiles: Signal::new(HashMap::new()),
        }
    }
}

fn on_connect() -> Vec<ClientMessage> {
    vec![
        ClientMessage::Identify {
            token: get_token(),
            nick: None,
        },
        ClientMessage::GetGameList,
    ]
}

pub(crate) fn use_state_provider(cx: &ScopeState) -> Signal<ClientState> {
    let state = *use_context_provider(cx, || Signal::new(ClientState::new()));
    let on_message = move |msg| {
        if !matches!(msg, ServerMessage::ServerTime(_)) {
            log::debug!("Received: {:?}", msg);
        }
        let state = state.read();
        match msg {
            ServerMessage::Identify {
                token,
                nick,
                user_id,
            } => {
                set_token(&token);
                state.user.set(Profile { user_id, nick });
            }
            ServerMessage::Profile(profile) => {
                state.profiles.write().insert(profile.user_id, profile);
            }
            _ => {}
        }
    };
    let _ = use_websocket_provider(cx, on_connect, on_message);
    state
}

fn get_token() -> Option<String> {
    gloo_storage::LocalStorage::get("token").ok()
}

fn set_token(token: &str) {
    gloo_storage::LocalStorage::set("token", token).unwrap();
}

pub(crate) fn set_nick(nick: &str) -> ClientMessage {
    ClientMessage::Identify {
        token: get_token(),
        nick: Some(nick.to_owned()),
    }
}
