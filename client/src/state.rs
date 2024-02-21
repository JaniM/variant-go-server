use std::{collections::HashMap, rc::Rc};

use crate::networking::use_websocket_provider;
use dioxus::prelude::*;
use dioxus_signals::{ReadOnlySignal, Signal};
use futures::StreamExt;
use gloo_storage::Storage as _;
use gloo_timers::future::TimeoutFuture;
use shared::{
    game::{self},
    message::{ClientMessage, Profile, ServerMessage},
};

#[derive(Clone, Copy)]
pub(crate) struct ClientState {
    pub(crate) user: Signal<Profile>,
    pub(crate) profiles: Signal<HashMap<u64, Profile>>,
    pub(crate) rooms: Signal<Vec<GameRoom>>,
    active_room: Signal<Option<ActiveRoom>>,
}

impl ClientState {
    fn new() -> Self {
        Self {
            user: Signal::new(Profile::default()),
            profiles: Signal::new(HashMap::new()),
            rooms: Signal::new(Vec::new()),
            active_room: Signal::new(None),
        }
    }

    pub(crate) fn active_room(&self) -> ReadOnlySignal<Option<ActiveRoom>> {
        ReadOnlySignal::new(self.active_room)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct GameRoom {
    pub(crate) id: u32,
    pub(crate) name: Rc<str>,
}

#[derive(Clone, Debug)]
pub(crate) struct ActiveRoom {
    pub(crate) id: u32,
    pub(crate) owner: u64,
    pub(crate) members: Vec<u64>,
    pub(crate) view: Rc<GameView>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameView {
    pub(crate) state: game::GameStateView,
    pub(crate) seats: Vec<game::Seat>,
    pub(crate) turn: u32,
    pub(crate) board: Vec<game::Color>,
    pub(crate) board_visibility: Option<Vec<u16>>,
    pub(crate) hidden_stones_left: u32,
    pub(crate) size: (u8, u8),
    pub(crate) mods: game::GameModifier,
    pub(crate) points: Vec<i32>,
    pub(crate) move_number: u32,
    pub(crate) clock: Option<game::clock::GameClock>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GameHistory {
    pub(crate) board: Vec<game::Color>,
    pub(crate) board_visibility: Option<Vec<u16>>,
    pub(crate) last_stone: Option<game::GroupVec<(u32, u32)>>,
    pub(crate) move_number: u32,
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

    let room_debouncer = use_debouncer(cx, 100, move |events| {
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
            ServerMessage::GameStatus {
                room_id,
                owner,
                members,
                seats,
                turn,
                board,
                board_visibility,
                hidden_stones_left,
                size,
                state: game_state,
                mods,
                points,
                move_number,
                clock,
            } => {
                let view = GameView {
                    state: game_state,
                    seats: seats
                        .into_iter()
                        .map(|(player, team, resigned)| game::Seat {
                            player,
                            team: game::Color(team),
                            resigned,
                        })
                        .collect(),
                    turn,
                    board: board.into_iter().map(game::Color).collect(),
                    board_visibility,
                    hidden_stones_left,
                    size,
                    mods,
                    points,
                    move_number,
                    clock,
                };
                let room = ActiveRoom {
                    id: room_id,
                    view: Rc::new(view),
                    owner,
                    members,
                };
                *state.active_room.write() = Some(room);
                log::debug!("{:?}", &*state.active_room.read());
            }
            _ => {}
        }
    };
    let _ = use_websocket_provider(cx, on_connect, on_message);
    state
}

pub(crate) fn use_state(cx: &ScopeState) -> Signal<ClientState> {
    *use_context(cx).expect("state not provided")
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

pub(crate) fn join_room(id: u32) -> ClientMessage {
    ClientMessage::JoinGame(id)
}

pub(crate) fn leave_all_rooms(state: Signal<ClientState>) -> ClientMessage {
    let active_room = state.read().active_room;
    *active_room.write() = None;
    ClientMessage::LeaveGame(None)
}

pub(crate) fn take_seat(id: u32) -> ClientMessage {
    ClientMessage::GameAction {
        room_id: None,
        action: shared::message::GameAction::TakeSeat(id),
    }
}

pub(crate) fn leave_seat(id: u32) -> ClientMessage {
    ClientMessage::GameAction {
        room_id: None,
        action: shared::message::GameAction::LeaveSeat(id),
    }
}

pub(crate) fn username(profile: &Profile) -> String {
    profile
        .nick
        .as_ref()
        .map_or_else(|| "Unknown".to_string(), |n| n.clone())
}
