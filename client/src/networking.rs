use dioxus::prelude::*;
use futures::{select, stream::FusedStream, SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_timers::future::TimeoutFuture;
use shared::message::{ClientMessage, ServerMessage};

use crate::config;

enum LoopFlow {
    Reconnect,
    Exit,
}

pub(crate) fn use_websocket_provider<'a>(
    cx: &'a ScopeState,
    mut on_connect: impl FnMut() -> Vec<ClientMessage> + 'static,
    mut receive: impl FnMut(ServerMessage) + 'static,
) -> impl Fn(ClientMessage) + 'a {
    use_coroutine(cx, move |rx: UnboundedReceiver<ClientMessage>| async move {
        log::info!("Connecting to WebSocket");
        let mut rx = rx.fuse();
        loop {
            let mut ws = match WebSocket::open(config::WS_URL) {
                Ok(ws) => ws,
                Err(e) => {
                    log::error!("Failed to connect to WebSocket: {}", e);
                    TimeoutFuture::new(config::CONN_RETRY_DELAY).await;
                    continue;
                }
            };
            for message in on_connect() {
                log::debug!("Sending initial message: {:?}", message);
                ws.send(pack(message)).await.unwrap();
            }
            match connection_loop(ws, &mut rx, &mut receive).await {
                LoopFlow::Reconnect => {
                    log::info!("Reconnecting to WebSocket");
                    TimeoutFuture::new(config::CONN_RETRY_DELAY).await;
                    continue;
                }
                LoopFlow::Exit => {
                    log::info!("Exiting WebSocket connection loop");
                    break;
                }
            }
        }
    });

    use_websocket(cx)
}

async fn connection_loop(
    ws: WebSocket,
    rx: &mut (impl FusedStream<Item = ClientMessage> + Unpin),
    receive: &mut impl FnMut(ServerMessage),
) -> LoopFlow {
    let (mut ws_sender, ws_recv) = ws.split();
    use gloo_net::websocket::Message;
    let mut ws_recv = ws_recv.fuse();
    loop {
        select! {
            msg = ws_recv.next() => {
                let Some(Ok(msg)) = msg else {
                    log::info!("{:?}", msg);
                    log::warn!("WebSocket closed");
                    return LoopFlow::Reconnect;
                };
                let msg: ServerMessage = match msg {
                    Message::Text(text) => {
                        log::warn!("Received text message: {}", text);
                        serde_cbor::from_slice(text.as_bytes()).unwrap()
                    }
                    Message::Bytes(bytes) => {
                        serde_cbor::from_slice(&bytes).unwrap()
                    }
                };
                receive(msg);
            }
            msg = rx.next() => {
                let Some(msg) = msg else {
                    log::error!("Client message channel closed");
                    return LoopFlow::Exit;
                };
                log::debug!("Sending: {:?}", msg);
                ws_sender.send(pack(msg)).await.unwrap();
            }
        }
    }
}

fn pack(msg: ClientMessage) -> gloo_net::websocket::Message {
    let bytes = serde_cbor::to_vec(&msg).expect("cbor fail");
    gloo_net::websocket::Message::Bytes(bytes)
}

pub(crate) fn use_websocket<'a>(cx: &'a ScopeState) -> impl Fn(ClientMessage) + 'a {
    let handle =
        use_coroutine_handle(cx).expect("use_websocket called outside of websocket provider");
    move |msg| handle.send(msg)
}
