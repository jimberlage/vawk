use actix::prelude::{Actor, Addr};
use actix_web;
use actix_web::http;
use actix_web::web;
use crate::command_executor::{Cancel, CommandExecutor, Connect, SetLineIndexFilters, Run};
use http::Method;
use std::io;

async fn connect(executor: web::Data<Addr<CommandExecutor>>) -> impl actix_web::Responder {
    match executor.send(Connect {}).await {
        Ok(connection) => {
            actix_web::HttpResponse::Ok()
                .header("Content-Type", "text/event-stream")
                .header("Access-Control-Allow-Origin", "http://localhost:3000")
                .streaming(connection)
        },
        Err(error) => {
            actix_web::HttpResponse::TooManyRequests().finish()
        },
    }
}

async fn run(executor: web::Data<Addr<CommandExecutor>>, web::Json(run): web::Json<Run>) -> impl actix_web::Responder {
    match executor.send(run).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

fn run_options() -> impl actix_web::Responder {
}

async fn cancel(executor: web::Data<Addr<CommandExecutor>>, web::Json(cancel): web::Json<Cancel>) -> impl actix_web::Responder {
    match executor.send(cancel).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

fn cancel_options() -> impl actix_web::Responder {
}

async fn set_line_index_filters(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_index_filters): web::Json<SetLineIndexFilters>) -> impl actix_web::Responder {
    match executor.send(set_line_index_filters).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

fn set_line_index_filters_options() -> impl actix_web::Responder {
}

pub async fn serve() -> io::Result<()> {
    let executor = CommandExecutor::new().start();
    let protected_executor = web::Data::new(executor);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .app_data(protected_executor.clone())
            .route("/api/connect", web::get().to(connect))
            .route("/api/command/run", web::post().to(run))
            .route("/api/command/run", web::method(Method::OPTIONS).to(run_options))
            .route("/api/command/cancel", web::post().to(cancel))
            .route("/api/command/cancel", web::method(Method::OPTIONS).to(cancel_options))
            .route("/api/line-index-filters", web::put().to(set_line_index_filters))
            .route("/api/line-index-filters", web::method(Method::OPTIONS).to(set_line_index_filters_options))
    })
    .bind("127.0.0.1:6846")?
    .run()
    .await
}
