use futures_util::{future, pin_mut, StreamExt};
use shared::message::ServerMessage;
use tokio::io::AsyncWriteExt;
use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};

use shared::message::{AdminAction, ClientMessage};

fn pack(msg: ClientMessage) -> Vec<u8> {
    serde_cbor::to_vec(&msg).unwrap()
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let url = url::Url::parse("ws://localhost:8088/ws/").unwrap();

    let (ws_tx, ws_rx) = futures_channel::mpsc::unbounded();

    tokio::spawn(read_stdin(ws_tx));

    let (ws_stream, _) = connect_async(url).await.expect("Failed to connect");
    println!("WebSocket handshake has been successfully completed");

    let (write, read) = ws_stream.split();

    let stdin_to_ws = ws_rx.map(Ok).forward(write);
    let ws_to_stdout = {
        read.for_each(|message| async {
            if let Ok(Message::Binary(data)) = message {
                let msg = serde_cbor::from_slice::<ServerMessage>(&data).unwrap();
                let mut stdout = tokio::io::stdout();
                stdout
                    .write_all(format!("{:?}\n", msg).as_bytes())
                    .await
                    .unwrap();
                stdout.flush().await.unwrap();
            }
        })
    };

    pin_mut!(stdin_to_ws, ws_to_stdout);
    future::select(stdin_to_ws, ws_to_stdout).await;
}

// Our helper method which will read data from stdin and send it along the
// sender provided.
async fn read_stdin(tx: futures_channel::mpsc::UnboundedSender<Message>) {
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
                _ => vec![],
            },
            _ => vec![],
        };

        for msg in msgs {
            tx.unbounded_send(Message::binary(pack(msg))).unwrap();
        }
    }
}
