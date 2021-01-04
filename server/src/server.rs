use actix::prelude::{Actor, Addr};
use actix_cors::Cors;
use actix_web;
use actix_web::web;
use crate::command_executor::{self, Cancel, CommandExecutor, Connect, Run};
use crate::parsers;
use serde::{Deserialize, Serialize};
use std::io;
use ulid::Ulid;

async fn connect(executor: web::Data<Addr<CommandExecutor>>) -> impl actix_web::Responder {
    match executor.send(Connect {}).await {
        Ok(connection) => {
            actix_web::HttpResponse::Ok()
                .header("Content-Type", "text/event-stream")
                .header("Access-Control-Allow-Origin", "http://localhost:3000")
                .streaming(connection)
        },
        Err(_error) => {
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

async fn cancel(executor: web::Data<Addr<CommandExecutor>>, web::Json(cancel): web::Json<Cancel>) -> impl actix_web::Responder {
    match executor.send(cancel).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

#[derive(Deserialize, Serialize)]
struct SetLineIndexFilters {
    client_id: Ulid,
    filters: Option<String>,
}

async fn set_line_index_filters(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_index_filters): web::Json<SetLineIndexFilters>) -> impl actix_web::Responder {
    match set_line_index_filters.filters.map(|filters| parsers::parse_index_filters(&filters)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(index_filters)) => {
            match executor.send(command_executor::SetLineIndexFilters {
                client_id: set_line_index_filters.client_id,
                filters: Some(index_filters),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetLineIndexFilters {
                client_id: set_line_index_filters.client_id,
                filters: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SetLineRegexFilter {
    client_id: Ulid,
    filter: Option<String>,
}

async fn set_line_regex_filter(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_regex_filter): web::Json<SetLineRegexFilter>) -> impl actix_web::Responder {
    match set_line_regex_filter.filter.map(|filter| parsers::parse_regex_filter(&filter)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(regex_filter)) => {
            match executor.send(command_executor::SetLineRegexFilter {
                client_id: set_line_regex_filter.client_id,
                filter: Some(regex_filter),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetLineRegexFilter {
                client_id: set_line_regex_filter.client_id,
                filter: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SetLineSeparators {
    client_id: Ulid,
    separators: Option<Vec<String>>,
}

async fn set_line_separators(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_separators): web::Json<SetLineSeparators>) -> impl actix_web::Responder {
    match set_line_separators.separators.map(|separators| parsers::parse_field_separators(&separators)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(separators)) => {
            match executor.send(command_executor::SetLineSeparators {
                client_id: set_line_separators.client_id,
                separators: Some(separators),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetLineSeparators {
                client_id: set_line_separators.client_id,
                separators: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SetRowIndexFilters {
    client_id: Ulid,
    filters: Option<String>,
}

async fn set_row_index_filters(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_index_filters): web::Json<SetRowIndexFilters>) -> impl actix_web::Responder {
    match set_row_index_filters.filters.map(|filters| parsers::parse_index_filters(&filters)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(index_filters)) => {
            match executor.send(command_executor::SetRowIndexFilters {
                client_id: set_row_index_filters.client_id,
                filters: Some(index_filters),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetRowIndexFilters {
                client_id: set_row_index_filters.client_id,
                filters: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SetRowRegexFilter {
    client_id: Ulid,
    filter: Option<String>,
}

async fn set_row_regex_filter(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_regex_filter): web::Json<SetRowRegexFilter>) -> impl actix_web::Responder {
    match set_row_regex_filter.filter.map(|filter| parsers::parse_regex_filter(&filter)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(regex_filter)) => {
            match executor.send(command_executor::SetRowRegexFilter {
                client_id: set_row_regex_filter.client_id,
                filter: Some(regex_filter),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetRowRegexFilter {
                client_id: set_row_regex_filter.client_id,
                filter: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct SetRowSeparators {
    client_id: Ulid,
    separators: Option<Vec<String>>,
}

async fn set_row_separators(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_separators): web::Json<SetRowSeparators>) -> impl actix_web::Responder {
    match set_row_separators.separators.map(|separators| parsers::parse_field_separators(&separators)) {
        // TODO: Give more of an update to the user here about what went wrong.
        Some(Err(_error)) => actix_web::HttpResponse::BadRequest().finish(),
        Some(Ok(separators)) => {
            match executor.send(command_executor::SetRowSeparators {
                client_id: set_row_separators.client_id,
                separators: Some(separators),
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        },
        None => {
            match executor.send(command_executor::SetRowSeparators {
                client_id: set_row_separators.client_id,
                separators: None,
            }).await {
                Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
                Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
                Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
            }
        }
    }
}

pub async fn serve() -> io::Result<()> {
    let executor = CommandExecutor::new().start();
    let protected_executor = web::Data::new(executor);

    actix_web::HttpServer::new(move || {
        actix_web::App::new()
            .wrap(
                // TODO: Restrict this to dev mode.
                Cors::default()
                    .allowed_origin("http://localhost:3000")
                    .allowed_methods(vec!["GET", "POST", "PUT"])
                    .max_age(3600),
            )
            .app_data(protected_executor.clone())
            .route("/api/connect", web::get().to(connect))
            .route("/api/command/run", web::post().to(run))
            .route("/api/command/cancel", web::post().to(cancel))
            .route("/api/line-index-filters", web::put().to(set_line_index_filters))
            .route("/api/line-regex-filter", web::put().to(set_line_regex_filter))
            .route("/api/line-separators", web::put().to(set_line_separators))
            .route("/api/row-index-filters", web::put().to(set_row_index_filters))
            .route("/api/row-regex-filter", web::put().to(set_row_regex_filter))
            .route("/api/row-separators", web::put().to(set_row_separators))
    })
    .bind("127.0.0.1:6846")?
    .run()
    .await
}
