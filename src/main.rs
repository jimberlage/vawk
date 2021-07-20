mod byte_trie;
mod parsers;
mod protos;
mod transformers;
mod websocket_connection;

use actix::clock;
use actix_cors::Cors;
use actix_files;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web_actors::ws;
use env_logger;
use std::io::{self, Read};
use std::process::Command;
use std::time::Duration;

fn open_gui(socket_address: &str) -> io::Result<()> {
    let mut child = Command::new("open")
        .arg(format!("http://{}", socket_address))
        .spawn()?;

    match child.wait() {
        Ok(exit_status) if exit_status.success() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::Other,
            "Got a nonzero exit code when opening the GUI.",
        )),
        Err(error) => Err(error),
    }
}

async fn connect(
    r: actix_web::HttpRequest,
    stream: web::Payload,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    ws::start(
        websocket_connection::WebsocketConnection::new(
            transformers::Options::default(),
            transformers::Options::default(),
        ),
        &r,
        stream,
    )
}

async fn run_server(stdin: &Vec<u8>, socket_address: &str) -> io::Result<()> {
    let html = include_bytes!("../ui/index.html");
    let js = include_bytes!("../ui/out.js");
    
    let server = actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .service(web::resource("/ws/").route(web::get().to(connect)))
            .service(actix_files::Files::new("/", "../client/").index_file("index.html"))
            .wrap(Logger::default())
            .wrap(Cors::permissive())
    })
    .bind(socket_address)?
    .run();

    // Give the server some time to spin up.
    // TODO: Add an on_running hook to actix-web.
    clock::delay_for(Duration::from_millis(150)).await;

    // Open the GUI.
    open_gui(socket_address)?;

    // And back to waiting for the server.
    server.await
}

#[actix_web::main]
async fn main() {
    env_logger::init();

    let mut stdin = vec![];
    if let Err(error) = io::stdin().read_to_end(&mut stdin) {
        log::error!("Failed to read command input:\n{}", error);
    }

    dbg!(String::from_utf8(stdin));

    // TODO: Add CLI helpers and configure port.
    let port = 6846;
    let socket_address = format!("127.0.0.1:{}", port);

    if let Err(error) = run_server(&vec![], &socket_address).await {
        log::error!("Failed to start server:\n{}", error);
    }
}
