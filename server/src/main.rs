
mod message;
mod server;

use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use crate::message::{ClientMessage, ServerMessage};
use crate::server::{ChatServer};

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    server_addr: web::Data<Addr<ChatServer>>
) -> Result<HttpResponse, Error> {
    println!("{:?}", r);
    let actor = MyWebSocket {
        hb: Instant::now(),
        id: 0,
        server_addr: server_addr.get_ref().clone(),
        room_id: None
    };
    let res = ws::start(actor, &r, stream);
    println!("{:?}", res);
    res
}

// TODO: see https://github.com/actix/examples/blob/master/websocket-chat/src/main.rs
// for how to implement socket <-> server communication

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct MyWebSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    id: usize,
    server_addr: Addr<ChatServer>,
    room_id: Option<u32>
}

impl Actor for MyWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of WsChatSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.server_addr
            .send(server::Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.server_addr.do_send(server::Disconnect { id: self.id });
        Running::Stop
    }
}


/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<server::Message> for MyWebSocket {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        match msg {
            server::Message::AnnounceRoom(_id) => {
                self.server_addr
                    .send(server::ListRooms)
                    .into_actor(self)
                    .then(|res, act, ctx| {
                        match res {
                            Ok(res) => ctx.binary(pack(ServerMessage::GameList { games: res })),
                            _ => ctx.stop()
                        }
                        fut::ready(())
                    })
                    .wait(ctx);
            },
            server::Message::GameStatus { room_id, moves } => {
                self.room_id = Some(room_id);
                ctx.binary(pack(ServerMessage::GameStatus { room_id, moves }));
            }
        };
    }
}

fn pack(msg: ServerMessage) -> Vec<u8> {
    serde_cbor::to_vec(&msg).expect("cbor fail")
}

/// Handler for `ws::Message`
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for MyWebSocket {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        // process websocket messages
        println!("WS: {:?}", msg);
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(text)) => ctx.text(text),
            Ok(ws::Message::Binary(bin)) => {
                match serde_cbor::from_slice::<ClientMessage>(&bin) {
                    Ok(ClientMessage::GetGameList) => {
                        self.server_addr
                            .send(server::ListRooms)
                            .into_actor(self)
                            .then(|res, act, ctx| {
                                match res {
                                    Ok(res) => ctx.binary(pack(ServerMessage::GameList { games: res })),
                                    _ => ctx.stop()
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    },
                    Ok(ClientMessage::StartGame) => {
                        self.server_addr.do_send(server::CreateRoom {
                            id: self.id
                        });
                    },
                    Ok(ClientMessage::JoinGame(room_id)) => {
                        self.server_addr.do_send(server::Join {
                            id: self.id,
                            room_id
                        });
                    },
                    Ok(ClientMessage::GameAction(action)) => {
                        if let Some(room_id) = self.room_id {
                            self.server_addr.do_send(server::GameAction {
                                id: self.id,
                                room_id,
                                action
                            });
                        }
                    },
                    Err(e) => ctx.binary(serde_cbor::to_vec(&ServerMessage::MsgError(format!("{}", e))).expect("cbor fail"))
                };
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl MyWebSocket {
    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut <Self as Actor>::Context) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    let server = ChatServer::default().start();

    HttpServer::new(move || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(server.clone())
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
    })
    // start http server on 127.0.0.1:8080
    .bind("127.0.0.1:8088")?
    .run()
    .await
}
