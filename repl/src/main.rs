use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

use futures_util::{future, pin_mut, StreamExt};
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use shared::message::{AdminAction, ClientMessage, ServerMessage};

#[derive(Default)]
struct RoomInfo {
    room_id: u32,
    players: Vec<u64>,
    member_count: usize,
    move_count: usize,
}

#[derive(Default)]
struct State {
    rooms: HashMap<u32, RoomInfo>,
}

fn pack(msg: ClientMessage) -> Vec<u8> {
    serde_cbor::to_vec(&msg).unwrap()
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let url = url::Url::parse("ws://localhost:8088/ws/").unwrap();

    let state = Arc::new(Mutex::new(State::default()));

    let (ws_tx, ws_rx) = futures_channel::mpsc::unbounded();

    tokio::spawn(read_stdin(state.clone(), ws_tx));

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws = ws_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        let state = state.clone();
        read.for_each(move |message| {
            if let Ok(Message::Binary(data)) = message {
                let msg = serde_cbor::from_slice::<ServerMessage>(&data).unwrap();
                #[allow(clippy::single_match)]
                match msg {
                    ServerMessage::GameStatus {
                        room_id,
                        move_number,
                        members,
                        seats,
                        ..
                    } => {
                        let mut state = state.lock().unwrap();
                        let room = state.rooms.entry(room_id).or_insert_with(RoomInfo::default);
                        room.room_id = room_id;
                        room.move_count = move_number as usize;
                        room.member_count = members.len() - 1; // Subtract self

                        room.players = seats.into_iter().filter_map(|x| x.0).collect();
                        room.players.sort_unstable();
                        room.players.dedup();

                        println!(
                            "Visited room {}: {} players, {} moves",
                            room_id, room.member_count, room.move_count
                        );
                    }
                    ServerMessage::AnnounceGame { room_id, name } => {
                        let mut state = state.lock().unwrap();
                        let room = state.rooms.entry(room_id).or_insert_with(RoomInfo::default);
                        room.room_id = room_id;
                        println!("New room {}: {:?}", room_id, name);
                    }
                    ServerMessage::CloseGame { room_id } => {
                        let mut state = state.lock().unwrap();
                        if state.rooms.remove(&room_id).is_some() {
                            println!("Closed room {}", room_id);
                        }
                    }
                    _ => {}
                }
            }
            async {}
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(state: Arc<Mutex<State>>, tx: futures_channel::mpsc::UnboundedSender<Message>) {
    use tokio::io::{self, AsyncBufReadExt, BufReader};

    let token = std::env::var("ADMIN_TOKEN").unwrap();
    tx.unbounded_send(Message::binary(pack(ClientMessage::Identify {
        token: Some(token.clone()),
        nick: None,
    })))
    .unwrap();

    let mut reader = BufReader::new(io::stdin());
    loop {
        let mut text = String::new();
        match reader.read_line(&mut text).await {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };

        let text = text.trim();

        let mut words = text.split(' ');
        let command = words.next().unwrap();

        let msgs = match command {
            "login" => vec![ClientMessage::Identify {
                token: Some(token.clone()),
                nick: None,
            }],
            "unload" | "ul" => match words.next() {
                Some("between") | Some("b") => {
                    let start: u32 = words.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    let end: u32 = words.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    (start..=end)
                        .map(|id| ClientMessage::Admin(AdminAction::UnloadRoom(id)))
                        .collect()
                }
                Some(x) if x.parse::<u32>().is_ok() => text
                    .split(' ')
                    .skip(1)
                    .filter_map(|x| x.parse::<u32>().ok())
                    .map(|id| ClientMessage::Admin(AdminAction::UnloadRoom(id)))
                    .collect(),
                _ => vec![],
            },
            "load" | "l" => match words.next() {
                Some("between") | Some("b") => {
                    let start: u32 = words.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    let end: u32 = words.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                    (start..=end).map(ClientMessage::JoinGame).collect()
                }
                Some(x) if x.parse::<u32>().is_ok() => text
                    .split(' ')
                    .skip(1)
                    .filter_map(|x| x.parse::<u32>().ok())
                    .map(ClientMessage::JoinGame)
                    .collect(),
                _ => vec![],
            },
            "list" | "li" => vec![ClientMessage::GetGameList],
            "visit" | "v" => {
                let state = state.lock().unwrap();
                state
                    .rooms
                    .values()
                    .map(|x| ClientMessage::JoinGame(x.room_id))
                    .collect()
            }
            "prune" | "p" => {
                let mut move_limit = None;
                let mut solo = false;
                while let Some(word) = words.next() {
                    match word {
                        "below" | "b" => {
                            let limit: usize =
                                words.next().and_then(|x| x.parse().ok()).unwrap_or(0);
                            move_limit = Some(limit);
                        }
                        "solo" | "s" => {
                            solo = true;
                        }
                        _ => break,
                    }
                }
                let state = state.lock().unwrap();
                state
                    .rooms
                    .values()
                    .filter(|x| {
                        (move_limit.is_none() || x.move_count < move_limit.unwrap_or(0))
                            && x.member_count == 0
                            && (!solo || x.players.len() < 2)
                    })
                    .map(|x| ClientMessage::Admin(AdminAction::UnloadRoom(x.room_id)))
                    .collect()
            }
            "quit" | "q" => std::process::exit(0),
            _ => vec![],
        };

        for msg in msgs {
            tx.unbounded_send(Message::binary(pack(msg))).unwrap();
        }
    }
}
