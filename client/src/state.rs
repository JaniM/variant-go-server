use std::{collections::HashMap, rc::Rc};

use crate::networking::use_websocket_provider;
use dioxus::prelude::*;
use dioxus_signals::Signal;
use futures::{select, FutureExt, StreamExt};
use gloo_storage::Storage as _;
use gloo_timers::future::TimeoutFuture;
use shared::message::{ClientMessage, Profile, ServerMessage};

#[derive(Clone, Copy)]
pub(crate) struct ClientState {
    pub(crate) user: Signal<Profile>,
    pub(crate) profiles: Signal<HashMap<u64, Profile>>,
    pub(crate) rooms: Signal<Vec<GameRoom>>,
}

impl ClientState {
    fn new() -> Self {
        Self {
            user: Signal::new(Profile::default()),
            profiles: Signal::new(HashMap::new()),
            rooms: Signal::new(Vec::new()),
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GameRoom {
    pub(crate) id: u32,
    pub(crate) name: Rc<str>,
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

enum RoomEvent {
    Announce(GameRoom),
    Close(u32),
}

fn use_debouncer<T: 'static>(
    cx: &ScopeState,
    debounce_time_ms: u32,
    mut callback: impl FnMut(Vec<T>) + 'static,
) -> impl Fn(T) {
    let (sender, rx) = cx.use_hook(|| {
        let (sender, rx) = futures::channel::mpsc::unbounded();
        (sender, Some(rx))
    });
    use_on_create(cx, move || {
        let mut rx = rx.take().unwrap();
        async move {
            loop {
                let mut queue = vec![];
                queue.push(rx.next().await.unwrap());
                let mut timed = (&mut rx).take_until(TimeoutFuture::new(debounce_time_ms));
                while let Some(value) = timed.next().await {
                    queue.push(value);
                }
                callback(queue);
            }
        }
    });
    let sender = sender.clone();
    move |value| sender.unbounded_send(value).expect("channel broken")
}

pub(crate) fn use_state_provider(cx: &ScopeState) -> Signal<ClientState> {
    let state = *use_context_provider(cx, || Signal::new(ClientState::new()));

    let room_debouncer = use_debouncer(cx, 10, move |events| {
        let rooms = state.read().rooms;
        let mut rooms = rooms.write();
        for event in events {
            apply_room_event(event, &mut *rooms);
        }
    });

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
            ServerMessage::AnnounceGame { room_id, name } => {
                let new_room = GameRoom {
                    id: room_id,
                    name: name.into(),
                };
                room_debouncer(RoomEvent::Announce(new_room));
            }
            ServerMessage::CloseGame { room_id } => {
                room_debouncer(RoomEvent::Close(room_id));
            }
            _ => {}
        }
    };
    let _ = use_websocket_provider(cx, on_connect, on_message);
    state
}

fn apply_room_event(event: RoomEvent, rooms: &mut Vec<GameRoom>) {
    match event {
        RoomEvent::Announce(room) => match rooms.binary_search_by(|r| room.id.cmp(&r.id)) {
            Ok(idx) => {
                log::warn!(
                    "Received a game we already knew about ({}, {:?})",
                    room.id,
                    room.name
                );
                rooms[idx] = room;
            }
            Err(idx) => rooms.insert(idx, room),
        },
        RoomEvent::Close(id) => match rooms.binary_search_by(|r| id.cmp(&r.id)) {
            Ok(idx) => {
                rooms.remove(idx);
            }
            Err(_) => {
                log::warn!("CloseGame on an unknown room ({})", id);
            }
        },
    }
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
