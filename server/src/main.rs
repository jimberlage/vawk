mod byte_trie;
mod parsers;
mod protos;
mod transformers;
mod websocket_connection;

use actix_cors::Cors;
use actix_files;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web_actors::ws;
use env_logger;
use std::io;

async fn connect(r: actix_web::HttpRequest, stream: web::Payload) -> Result<actix_web::HttpResponse, actix_web::Error> {
    ws::start(websocket_connection::WebsocketConnection::new(transformers::Options::default(), transformers::Options::default()), &r, stream)
}

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .service(web::resource("/ws/").route(web::get().to(connect)))
            .service(actix_files::Files::new("/", "../client/").index_file("index.html"))
            .wrap(Logger::default())
            .wrap(Cors::permissive())
    })
    .bind("127.0.0.1:6846")?
    .run()
    .await
}
