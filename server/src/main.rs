mod byte_trie;
mod command_executor;
mod encoding;
mod parsers;
mod server;
mod transformers;

use actix::prelude::Actor;
use actix_cors::Cors;
use actix_web::middleware::Logger;
use actix_web::web;
use crate::command_executor::CommandExecutor;
use std::io;

#[actix_web::main]
async fn main() -> io::Result<()> {
    env_logger::init();

    let executor = CommandExecutor::new().start();
    let protected_executor = web::Data::new(executor);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .route("/api/command/cancel", web::post().to(server::cancel))
            .route("/api/command/run", web::post().to(server::run))
            .route("/api/connect", web::get().to(server::connect))
            .route("/api/line-index-filters", web::put().to(server::set_line_index_filters))
            .route("/api/line-regex-filter", web::put().to(server::set_line_regex_filter))
            .route("/api/line-separators", web::put().to(server::set_line_separators))
            .route("/api/listen", web::get().to(server::listen))
            .route("/api/row-index-filters", web::put().to(server::set_row_index_filters))
            .route("/api/row-regex-filter", web::put().to(server::set_row_regex_filter))
            .route("/api/row-separators", web::put().to(server::set_row_separators))
            .app_data(protected_executor.clone())
            .wrap(Logger::default())
            .wrap(
                // TODO: Restrict this to dev mode.
                Cors::permissive()
            )
    })
    .bind("127.0.0.1:6846")?
    .run()
    .await
}
