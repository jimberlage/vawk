mod byte_trie;
mod parsers;
mod protos;
mod transformers;
mod websocket_connection;

use actix::clock;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web;
use actix_web_actors::ws;
use clap::{App, Arg};
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

struct Context {
    bundled_html: String,
    bundled_css: String,
    bundled_js: String,
    bundled_js_map: String,
    stdin: Vec<u8>,
}

async fn connect(
    r: actix_web::HttpRequest,
    stream: web::Payload,
    context: web::Data<Context>,
) -> Result<actix_web::HttpResponse, actix_web::Error> {
    ws::start(
        websocket_connection::WebsocketConnection::new(
            context.stdin.clone(),
            transformers::Options::default(),
            transformers::Options::default(),
        ),
        &r,
        stream,
    )
}

#[actix_web::get("/")]
async fn index(context: web::Data<Context>) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("text/html")
        .body(context.bundled_html.clone())
}

#[actix_web::get("/out.css")]
async fn index_css(context: web::Data<Context>) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("text/css")
        .body(context.bundled_css.clone())
}

#[actix_web::get("/out.js")]
async fn index_js(context: web::Data<Context>) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("text/javascript")
        .body(context.bundled_js.clone())
}

#[actix_web::get("/out.js.map")]
async fn index_js_map(context: web::Data<Context>) -> impl actix_web::Responder {
    actix_web::HttpResponse::Ok()
        .content_type("application/octet-stream")
        .body(context.bundled_js_map.clone())
}

async fn run_server(
    stdin: Vec<u8>,
    socket_address: &str,
    in_development_mode: bool,
) -> io::Result<()> {
    let html = include_str!("../ui/index.html");
    let css = include_str!("../ui/out.css");
    let js = include_str!("../ui/out.js");
    let js_map = include_str!("../ui/out.js.map");

    let server = actix_web::HttpServer::new(move || {
        let mut app = actix_web::App::new()
            .data(Context {
                bundled_html: html.to_owned(),
                bundled_css: css.to_owned(),
                bundled_js: js.to_owned(),
                bundled_js_map: js_map.to_owned(),
                stdin: stdin.clone(),
            })
            .service(web::resource("/ws/").route(web::get().to(connect)));

        app = if in_development_mode {
            app.service(actix_files::Files::new("/", "./ui/").index_file("index.html"))
        } else {
            app.service(index).service(index_css).service(index_js).service(index_js_map)
        };

        app.wrap(Logger::default()).wrap(Cors::permissive())
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

    let matches = App::new("VAWK (Visual AWK)")
        .version("1.0")
        .author("Jim Berlage <jamesberlage@gmail.com>")
        .about("Allows users to view process output as a spreadsheet.")
        .arg(
            Arg::with_name("development-mode")
                .long("development-mode")
                .help(
                    "Allows the user to run the frontend using esbuild for more rapid development.",
                )
                .takes_value(false)
                .required(false),
        )
        .get_matches();
    let in_development_mode = matches.is_present("development-mode");

    let mut stdin = vec![];
    if let Err(error) = io::stdin().read_to_end(&mut stdin) {
        log::error!("Failed to read command input:\n{}", error);
    }

    // TODO: Add CLI helpers and configure port.
    let port = 6846;
    let socket_address = format!("127.0.0.1:{}", port);

    if let Err(error) = run_server(stdin, &socket_address, in_development_mode).await {
        log::error!("Failed to start server:\n{}", error);
    }
}
