mod game;
mod game_room;
mod message;
mod server;

use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use crate::message::{ClientMessage, ServerMessage};
use crate::server::GameServer;

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    server_addr: web::Data<Addr<GameServer>>,
) -> Result<HttpResponse, Error> {
    println!("{:?}", r);
    let actor = ClientWebSocket {
        hb: Instant::now(),
        id: 0,
        server_addr: server_addr.get_ref().clone(),
        game_addr: None,
        room_id: None,
    };
    let res = ws::start(actor, &r, stream);
    println!("{:?}", res);
    res
}

// TODO: see https://github.com/actix/examples/blob/master/websocket-chat/src/main.rs
// for how to implement socket <-> server communication

/// websocket connection is long running connection, it easier
/// to handle with an actor
struct ClientWebSocket {
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    id: usize,
    server_addr: Addr<GameServer>,
    game_addr: Option<Addr<game_room::GameRoom>>,
    room_id: Option<u32>,
}

impl Actor for ClientWebSocket {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start. We start the heartbeat process here.
    fn started(&mut self, ctx: &mut Self::Context) {
        self.hb(ctx);

        // register self in game server.
        let addr = ctx.address();
        self.server_addr
            .send(server::Connect {
                addr: addr.clone().recipient(),
                game_addr: addr.recipient(),
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

impl Handler<game_room::Message> for ClientWebSocket {
    type Result = ();

    fn handle(&mut self, msg: game_room::Message, ctx: &mut Self::Context) {
        match msg {
            game_room::Message::GameStatus {
                room_id,
                members,
                view,
            } => {
                ctx.binary(pack(ServerMessage::GameStatus {
                    room_id,
                    members,
                    seats: view
                        .seats
                        .into_iter()
                        .map(|x| (x.player, x.team.0))
                        .collect(),
                    turn: view.turn,
                    board: view.board.into_iter().map(|x| x.0).collect(),
                    size: view.size,
                    state: view.state,
                    mods: view.mods,
                }));
            }
        }
    }
}

impl Handler<server::Message> for ClientWebSocket {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        match msg {
            server::Message::AnnounceRoom(room_id, name) => {
                ctx.binary(pack(ServerMessage::AnnounceGame { room_id, name }));
            }
            server::Message::CloseRoom(room_id) => {
                ctx.binary(pack(ServerMessage::CloseGame { room_id }));
            }
            server::Message::Identify(res) => {
                ctx.binary(pack(ServerMessage::Identify {
                    user_id: res.user_id,
                    token: res.token.to_string(),
                    nick: res.nick,
                }));
            }
            server::Message::UpdateProfile(res) => {
                ctx.binary(pack(ServerMessage::Profile(message::Profile {
                    user_id: res.user_id,
                    nick: res.nick,
                })));
            }
        };
    }
}

fn pack(msg: ServerMessage) -> Vec<u8> {
    serde_cbor::to_vec(&msg).expect("cbor fail")
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ClientWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
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
                let data = serde_cbor::from_slice::<ClientMessage>(&bin);
                println!("WS: {:?}", data);
                match data {
                    Ok(ClientMessage::GetGameList) => {
                        self.server_addr
                            .send(server::ListRooms)
                            .into_actor(self)
                            .then(|res, _act, ctx| {
                                match res {
                                    Ok(mut res) => {
                                        // Sort newest first
                                        res.sort_unstable_by_key(|x| -(x.0 as i32));
                                        for (room_id, name) in res {
                                            ctx.binary(pack(ServerMessage::AnnounceGame {
                                                room_id,
                                                name,
                                            }));
                                        }
                                    }
                                    _ => ctx.stop(),
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    Ok(ClientMessage::StartGame {
                        name,
                        seats,
                        komis,
                        size,
                        mods,
                    }) => {
                        self.server_addr
                            .send(server::CreateRoom {
                                id: self.id,
                                name,
                                seats,
                                komis,
                                size,
                                mods,
                            })
                            .into_actor(self)
                            .then(|res, act, _| {
                                if let Ok(Ok((id, addr))) = res {
                                    act.room_id = Some(id);
                                    act.game_addr = Some(addr);
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    Ok(ClientMessage::JoinGame(room_id)) => {
                        self.server_addr
                            .send(server::Join {
                                id: self.id,
                                room_id,
                            })
                            .into_actor(self)
                            .then(move |res, act, _| {
                                if let Ok(Ok(addr)) = res {
                                    act.room_id = Some(room_id);
                                    act.game_addr = Some(addr);
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    Ok(ClientMessage::GameAction(action)) => {
                        if let Some(addr) = &self.game_addr {
                            addr.do_send(game_room::GameAction {
                                id: self.id,
                                action,
                            });
                        }
                    }
                    Ok(ClientMessage::Identify { token, nick }) => {
                        self.server_addr
                            .send(server::IdentifyAs {
                                id: self.id,
                                token,
                                nick,
                            })
                            .into_actor(self)
                            .then(|res, _act, ctx| {
                                match res {
                                    Ok(res) => ctx.binary(pack(ServerMessage::Identify {
                                        user_id: res.user_id,
                                        token: res.token.to_string(),
                                        nick: res.nick,
                                    })),
                                    _ => ctx.stop(),
                                }
                                fut::ready(())
                            })
                            .wait(ctx);
                    }
                    Err(e) => ctx.binary(
                        serde_cbor::to_vec(&ServerMessage::MsgError(format!("{}", e)))
                            .expect("cbor fail"),
                    ),
                };
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            _ => ctx.stop(),
        }
    }
}

impl ClientWebSocket {
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

    let server = GameServer::default().start();

    HttpServer::new(move || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(server.clone())
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
    })
    .bind("0.0.0.0:8088")?
    .run()
    .await
}
