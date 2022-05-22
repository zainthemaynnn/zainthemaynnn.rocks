mod server;

use actix::{prelude::*, Actor, StreamHandler};
use actix_files::NamedFile;
use actix_web::{get, web, App, HttpRequest, HttpResponse, HttpServer};
use actix_web_actors::ws;
use serde::{Deserialize, Serialize};
use std::{env, fmt::Debug, net::Ipv4Addr};

#[get("/")]
async fn index(_: web::Path<()>) -> actix_web::Result<NamedFile> {
    Ok(NamedFile::open("./static/index.html")?)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
enum CanvasMessage {
    Draw(server::Draw),
}

#[derive(Debug)]
struct Client {
    server: Addr<server::Server>,
}

impl Actor for Client {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        self.server
            .send(server::Connect {
                client: ctx.address().recipient(),
            })
            .into_actor(self)
            .then(|res, _act, ctx| {
                match res {
                    Ok(res) => ctx.binary(res),
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }
}

impl Handler<server::Draw> for Client {
    type Result = ();

    fn handle(&mut self, msg: server::Draw, ctx: &mut Self::Context) -> Self::Result {
        ctx.text(serde_json::to_string(&CanvasMessage::Draw(msg)).unwrap());
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for Client {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        let msg = match msg {
            Ok(msg) => msg,
            Err(_) => {
                ctx.close(None);
                return;
            }
        };

        match msg {
            ws::Message::Text(msg) => {
                if let Ok(msg) = serde_json::from_str::<CanvasMessage>(&msg) {
                    match msg {
                        CanvasMessage::Draw(draw) => {
                            self.server.do_send(draw);
                        }
                    };
                }
            }
            ws::Message::Close(r) => {
                self.server.do_send(server::Disconnect {
                    client: ctx.address().recipient(),
                });
                ctx.close(r);
                ctx.stop();
            }
            ws::Message::Continuation(_) => ctx.stop(),
            ws::Message::Binary(_) | ws::Message::Nop => (),
            _ => (),
        };
    }
}

#[get("/ws")]
async fn socket(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<server::Server>>,
) -> Result<HttpResponse, actix_web::Error> {
    ws::start(
        Client {
            server: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let srv = (server::Server::new()).start();

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(srv.clone()))
            .service(index)
            .service(socket)
    })
    .bind((
        Ipv4Addr::new(0, 0, 0, 0),
        env::var("PORT")
            .unwrap_or_else(|_| String::from("3000"))
            .parse()
            .expect("$PORT must be a number"),
    ))?
    .run()
    .await
}
