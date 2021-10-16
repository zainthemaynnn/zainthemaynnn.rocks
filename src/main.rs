use actix_web::{get, web, App, HttpServer, Responder};

#[get("/")]
async fn index(_: web::Path<()>) -> impl Responder {
    String::from(":)")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().service(index))
        .bind("localhost:80")?
        .run()
        .await
}
