use actix::prelude::Addr;
use actix_web;
use actix_web::HttpResponse;
use actix_web::web;
use crate::command_executor::{self, Cancel, CommandExecutor, Connect, Listen, Run};
use crate::parsers;
use serde::{Deserialize, Serialize};
use ulid::Ulid;

pub async fn listen(executor: web::Data<Addr<CommandExecutor>>, web::Query(listen_msg): web::Query<Listen>) -> HttpResponse {
    match executor.send(listen_msg).await {
        Ok(Ok(connection)) => {
            actix_web::HttpResponse::Ok()
                .header("Content-Type", "text/event-stream")
                .header("Access-Control-Allow-Origin", "http://localhost:3000")
                .streaming(connection)
        },
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

pub async fn connect(executor: web::Data<Addr<CommandExecutor>>) -> HttpResponse {
    match executor.send(Connect {}).await {
        Ok(response) => actix_web::HttpResponse::Ok().json(response),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

pub async fn run(executor: web::Data<Addr<CommandExecutor>>, web::Json(run): web::Json<Run>) -> HttpResponse {
    match executor.send(run).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

pub async fn cancel(executor: web::Data<Addr<CommandExecutor>>, web::Json(cancel): web::Json<Cancel>) -> HttpResponse {
    match executor.send(cancel).await {
        Ok(Ok(())) => actix_web::HttpResponse::Ok().finish(),
        Ok(Err(_)) => actix_web::HttpResponse::BadRequest().finish(),
        Err(_) => actix_web::HttpResponse::TooManyRequests().finish(),
    }
}

#[derive(Deserialize, Serialize)]
pub struct SetLineIndexFilters {
    pub client_id: Ulid,
    pub filters: Option<String>,
}

pub async fn set_line_index_filters(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_index_filters): web::Json<SetLineIndexFilters>) -> HttpResponse {
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
pub struct SetLineRegexFilter {
    pub client_id: Ulid,
    pub filter: Option<String>,
}

pub async fn set_line_regex_filter(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_regex_filter): web::Json<SetLineRegexFilter>) -> HttpResponse {
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
pub struct SetLineSeparators {
    pub client_id: Ulid,
    pub separators: Option<Vec<String>>,
}

pub async fn set_line_separators(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_line_separators): web::Json<SetLineSeparators>) -> HttpResponse {
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
pub struct SetRowIndexFilters {
    pub client_id: Ulid,
    pub filters: Option<String>,
}

pub async fn set_row_index_filters(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_index_filters): web::Json<SetRowIndexFilters>) -> HttpResponse {
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
pub struct SetRowRegexFilter {
    pub client_id: Ulid,
    pub filter: Option<String>,
}

pub async fn set_row_regex_filter(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_regex_filter): web::Json<SetRowRegexFilter>) -> HttpResponse {
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
pub struct SetRowSeparators {
    pub client_id: Ulid,
    pub separators: Option<Vec<String>>,
}

pub async fn set_row_separators(executor: web::Data<Addr<CommandExecutor>>, web::Json(set_row_separators): web::Json<SetRowSeparators>) -> HttpResponse {
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
