#[macro_use]
extern crate diesel;

mod db;
mod game_room;
mod schema;
mod server;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use actix::prelude::*;
use actix_web::{middleware, web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;

use crate::server::GameServer;
use shared::message::{self, ClientMessage, ClientMode, ServerMessage};

use serde::{Deserialize, Serialize};

macro_rules! catch {
    ($($code:tt)+) => {
        (|| Some({ $($code)+ }))()
    };
}

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// How often time syncs are sent
const TIMESYNC_INTERVAL: Duration = Duration::from_secs(2);

/// do websocket handshake and start `MyWebSocket` actor
async fn ws_index(
    r: HttpRequest,
    stream: web::Payload,
    server_addr: web::Data<Addr<GameServer>>,
) -> Result<HttpResponse, Error> {
    let actor = ClientWebSocket {
        hb: Instant::now(),
        id: 0,
        server_addr: server_addr.get_ref().clone(),
        game_addr: HashMap::new(),
        room_id: None,
        mode: ClientMode::Client,
        ratelimit_hb: Instant::now(),
        ratelimit_counter: 0,
        ratelimit_block_target: None,
        is_admin: false,
    };
    ws::start(actor, &r, stream)
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
    game_addr: HashMap<u32, Addr<game_room::GameRoom>>,
    room_id: Option<u32>,
    mode: ClientMode,

    ratelimit_hb: Instant,
    ratelimit_counter: u64,
    ratelimit_block_target: Option<Instant>,

    is_admin: bool,
}

type Context = ws::WebsocketContext<ClientWebSocket>;

impl Actor for ClientWebSocket {
    type Context = Context;

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
                owner,
                members,
                view,
            } => {
                ctx.binary(
                    ServerMessage::GameStatus {
                        room_id,
                        owner,
                        members,
                        seats: view
                            .seats
                            .into_iter()
                            .map(|x| (x.player, x.team.0, x.resigned))
                            .collect(),
                        turn: view.turn,
                        board: view.board.into_iter().map(|x| x.0).collect(),
                        board_visibility: view.board_visibility,
                        hidden_stones_left: view.hidden_stones_left,
                        size: view.size,
                        state: view.state,
                        mods: view.mods,
                        points: view.points.to_vec(),
                        move_number: view.move_number,
                        clock: view.clock,
                    }
                    .pack(),
                );
            }
            game_room::Message::BoardAt { view, room_id } => {
                ctx.binary(ServerMessage::BoardAt { view, room_id }.pack());
            }
            game_room::Message::SGF { sgf, room_id } => {
                ctx.binary(ServerMessage::SGF { sgf, room_id }.pack());
            }
        }
    }
}

impl Handler<server::Message> for ClientWebSocket {
    type Result = ();

    fn handle(&mut self, msg: server::Message, ctx: &mut Self::Context) {
        match msg {
            server::Message::AnnounceRoom(room_id, name) => {
                ctx.binary(ServerMessage::AnnounceGame { room_id, name }.pack());
            }
            server::Message::CloseRoom(room_id) => {
                ctx.binary(ServerMessage::CloseGame { room_id }.pack());
            }
            server::Message::Identify(res) => {
                ctx.binary(
                    ServerMessage::Identify {
                        user_id: res.user_id,
                        token: res.token.to_string(),
                        nick: res.nick,
                    }
                    .pack(),
                );
            }
            server::Message::UpdateProfile(res) => {
                ctx.binary(
                    ServerMessage::Profile(message::Profile {
                        user_id: res.user_id,
                        nick: res.nick,
                    })
                    .pack(),
                );
            }
        };
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ClientWebSocket {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        use std::convert::TryInto;

        let now = Instant::now();

        if !self.is_admin {
            let diff = now - self.ratelimit_hb;
            // Subtract the counter by 1 every 100ms
            self.ratelimit_counter = self
                .ratelimit_counter
                .saturating_sub((diff.as_millis() / 100).try_into().unwrap());
            self.ratelimit_hb = now;

            if let Some(target) = self.ratelimit_block_target {
                if now < target {
                    ctx.binary(ServerMessage::Error(message::Error::RateLimit).pack());
                    return;
                }
                self.ratelimit_block_target = None;
            }

            self.ratelimit_counter += 1;

            // Allow max 10 messages per second
            if self.ratelimit_counter > 10 {
                self.ratelimit_block_target = Some(now + Duration::from_secs(2));
                return;
            }
        }

        match msg {
            Ok(ws::Message::Ping(msg)) => {
                self.hb = now;
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                self.hb = Instant::now();
            }
            Ok(ws::Message::Text(_)) => {}
            Ok(ws::Message::Binary(bin)) => {
                let data = serde_cbor::from_slice::<ClientMessage>(&bin);
                match data {
                    Ok(data) => self.handle_message(data, ctx),
                    Err(e) => ctx.binary(ServerMessage::MsgError(format!("{}", e)).pack()),
                }
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
    fn hb(&self, ctx: &mut Context) {
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

        ctx.run_interval(TIMESYNC_INTERVAL, |_act, ctx| {
            ctx.binary(ServerMessage::ServerTime(shared::game::clock::Millisecond::now()).pack());
        });
    }

    fn handle_get_game_list(&mut self, ctx: &mut Context) {
        fn send_rooms(mut rooms: Vec<(u32, String)>, ctx: &mut Context) {
            // Sort newest first
            rooms.sort_unstable_by_key(|x| -(x.0 as i32));
            for (room_id, name) in rooms {
                ctx.binary(ServerMessage::AnnounceGame { room_id, name }.pack());
            }
        }

        self.server_addr
            .send(server::ListRooms)
            .into_actor(self)
            .then(|res, _act, ctx| {
                match res {
                    Ok(res) => send_rooms(res, ctx),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn handle_start_game(&mut self, msg: message::StartGame, ctx: &mut Context) {
        self.server_addr
            .send(server::CreateRoom {
                id: self.id,
                room: msg,
                leave_previous: match self.mode {
                    ClientMode::Client => true,
                    ClientMode::Integration => false,
                },
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(Ok((id, addr))) => {
                        act.room_id = Some(id);
                        act.game_addr.insert(id, addr.unwrap());
                    }
                    Ok(Err(err)) => {
                        ctx.binary(ServerMessage::Error(err).pack());
                    }
                    _ => {}
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn handle_join_game(&mut self, room_id: u32, ctx: &mut Context) {
        self.server_addr
            .send(server::Join {
                id: self.id,
                room_id,
                leave_previous: match self.mode {
                    ClientMode::Client => true,
                    ClientMode::Integration => false,
                },
            })
            .into_actor(self)
            .then(move |res, act, _| {
                if let Ok(Ok(addr)) = res {
                    act.room_id = Some(room_id);
                    act.game_addr.insert(room_id, addr);
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn handle_leave_game(&mut self, room_id: Option<u32>, ctx: &mut Context) {
        self.server_addr
            .send(server::LeaveRoom {
                id: self.id,
                room_id,
            })
            .into_actor(self)
            .then(move |_res, _act, _| fut::ready(()))
            .wait(ctx);
    }

    fn handle_identify(&mut self, token: Option<String>, nick: Option<String>, ctx: &mut Context) {
        self.server_addr
            .send(server::IdentifyAs {
                id: self.id,
                token,
                nick,
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(Ok(res)) => {
                        act.is_admin = res.is_admin;
                        ctx.binary(
                            ServerMessage::Identify {
                                user_id: res.user_id,
                                token: res.token.to_string(),
                                nick: res.nick,
                            }
                            .pack(),
                        )
                    }
                    Ok(Err(err)) => {
                        ctx.binary(ServerMessage::Error(err).pack());
                    }
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn handle_message(&mut self, msg: ClientMessage, ctx: &mut Context) {
        println!("WS: {:?}", msg);
        match msg {
            ClientMessage::GetGameList => {
                self.handle_get_game_list(ctx);
            }
            ClientMessage::StartGame(start) => {
                self.handle_start_game(start, ctx);
            }
            ClientMessage::JoinGame(room_id) => {
                self.handle_join_game(room_id, ctx);
            }
            ClientMessage::LeaveGame(room_id) => {
                self.handle_leave_game(room_id, ctx);
            }
            ClientMessage::GameAction { room_id, action } => {
                if let Some(addr) = &self.game_addr.get(&room_id.or(self.room_id).unwrap_or(0)) {
                    addr.send(game_room::GameAction {
                        id: self.id,
                        action,
                    })
                    .into_actor(self)
                    .then(|res, _act, ctx| {
                        match res {
                            Ok(Ok(())) => {}
                            Ok(Err(err)) => {
                                ctx.binary(ServerMessage::Error(err).pack());
                            }
                            _ => {}
                        }
                        fut::ready(())
                    })
                    .wait(ctx);
                }
            }
            ClientMessage::Identify { token, nick } => {
                self.handle_identify(token, nick, ctx);
            }
            ClientMessage::Admin(action) => {
                self.server_addr.do_send(server::AdminMessage {
                    client_id: self.id,
                    action,
                });
            }
            ClientMessage::Mode(mode) => {
                self.mode = mode;
            }
        };
    }
}

#[derive(Debug, Deserialize)]
struct CreateGameBody {
    game: message::StartGame,
    #[serde(default)]
    players: Option<Vec<Option<u64>>>,
}

#[derive(Debug, Serialize)]
struct CreateGameResponse {
    id: u32,
}

async fn create_game(
    req: actix_web::HttpRequest,
    body: web::Json<CreateGameBody>,
    server_addr: web::Data<Addr<GameServer>>,
    db_addr: web::Data<Addr<db::DbActor>>,
) -> actix_web::Result<HttpResponse> {
    println!("POST /game/create: {:?}", body);

    let token = match catch! {
        let header = req.headers().get("Authentication")?;
        header.to_str().ok()?.to_owned()
    } {
        Some(x) => x,
        None => return Ok(HttpResponse::BadRequest().body("Bearer token required")),
    };

    let user = match db_addr.send(db::GetUserByToken(token)).await? {
        Ok(x) => x,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Invalid token")),
    };

    if !user.has_integration_access {
        return Ok(HttpResponse::BadRequest().body("Invalid token"));
    }

    let CreateGameBody { game, players } = body.into_inner();

    let resp = server_addr
        .send(server::CreateRoom {
            id: 0,
            room: game,
            leave_previous: false,
        })
        .await?;

    let (id, addr) = match resp {
        Ok((id, Some(addr))) => (id, addr),
        _ => return Ok(HttpResponse::BadRequest().body("Game creation error")),
    };

    for (idx, &user_id) in players.iter().flatten().enumerate() {
        if let Some(user_id) = user_id {
            addr.do_send(game_room::GameActionAsUser {
                user_id,
                action: message::GameAction::TakeSeat(idx as _),
            });
        }
    }

    Ok(HttpResponse::Ok().json(CreateGameResponse { id }))
}

#[derive(Debug, Serialize)]
struct GetGameResponse {
    game: shared::game::GameView,
}

async fn get_game_view(
    req: actix_web::HttpRequest,
    server_addr: web::Data<Addr<GameServer>>,
) -> actix_web::Result<HttpResponse> {
    let room_id = req.match_info().get("id").unwrap().parse().unwrap();

    let resp = server_addr.send(server::GetAdminView { room_id }).await?;

    let view = match resp {
        Ok(view) => view,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Game fetch error")),
    };

    Ok(HttpResponse::Ok().json(GetGameResponse { game: view }))
}

#[derive(Debug, Serialize)]
enum GameState {
    Play,
    Done,
}

#[derive(Debug, Serialize)]
struct TeamResult {
    score: f32,
    resigned: bool,
}

#[derive(Debug, Serialize)]
struct GetGameResultResponse {
    state: GameState,
    teams: Option<Vec<TeamResult>>,
    winner: Option<usize>,
}

async fn get_game_result(
    req: actix_web::HttpRequest,
    server_addr: web::Data<Addr<GameServer>>,
) -> actix_web::Result<HttpResponse> {
    use shared::game::GameStateView;

    let room_id = req.match_info().get("id").unwrap().parse().unwrap();

    let resp = server_addr.send(server::GetAdminView { room_id }).await?;

    let view = match resp {
        Ok(view) => view,
        Err(_) => return Ok(HttpResponse::BadRequest().body("Game fetch error")),
    };

    let response = match &view.state {
        GameStateView::Done(scoring) => {
            let mut teams: Vec<_> = scoring
                .scores
                .iter()
                .map(|&s| TeamResult {
                    score: s as f32 / 2.0,
                    resigned: false,
                })
                .collect();
            for seat in &view.seats {
                teams[seat.team.as_usize() - 1].resigned |= seat.resigned;
            }
            let mut winner = (0, 0.0);
            for (idx, team) in teams.iter().enumerate() {
                if team.resigned {
                    continue;
                }
                if team.score > winner.1 {
                    winner = (idx + 1, team.score);
                }
            }
            GetGameResultResponse {
                state: GameState::Done,
                teams: Some(teams),
                winner: Some(winner.0),
            }
        }
        _ => GetGameResultResponse {
            state: GameState::Play,
            teams: None,
            winner: None,
        },
    };

    Ok(HttpResponse::Ok().json(response))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info");
    env_logger::init();

    let server = GameServer::default().start();
    let db = SyncArbiter::start(1, db::DbActor::default);

    HttpServer::new(move || {
        App::new()
            // enable logger
            .wrap(middleware::Logger::default())
            .data(server.clone())
            .data(db.clone())
            // websocket route
            .service(web::resource("/ws/").route(web::get().to(ws_index)))
            .service(web::resource("/api/game/create").route(web::post().to(create_game)))
            .service(web::resource("/api/game/{id}").route(web::get().to(get_game_view)))
            .service(web::resource("/api/game/{id}/result").route(web::get().to(get_game_result)))
    })
    .bind("0.0.0.0:8088")?
    .run()
    .await
}
