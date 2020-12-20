#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
extern crate nom;
extern crate regex;
#[macro_use]
extern crate rocket;
extern crate serde;
extern crate serde_json;

mod parser;

use rocket::http::hyper::header::Connection;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::State;
use rocket_contrib::json::Json;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::io::{self, BufReader, Read, Write};
use std::process::{self, Output};
use std::sync::{Arc, Mutex};

struct Message(Output);

impl Serialize for Message {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut data = serializer.serialize_struct("data", 3)?;
        data.serialize_field("status", &self.0.status.code())?;
        data.serialize_field("stdout", &base64::encode(&self.0.stdout))?;
        data.serialize_field("stderr", &base64::encode(&self.0.stderr))?;
        data.end()
    }
}

impl Message {
    fn to_server_side_event(&self) -> serde_json::Result<Vec<u8>> {
        let serialized = serde_json::to_vec(self)?;
        Ok(["data: ".as_bytes(), &serialized, "\n\n".as_bytes()].concat())
    }
}

#[derive(Clone, Debug, Deserialize)]
struct Command {
    command: String,
}

#[derive(Clone, Debug)]
struct DB {
    command: Option<Command>,
    refresh_requested: bool,
}

struct CORSPreflightResponder {
    allowed_origins: Vec<String>,
    max_age: usize,
}

impl<'a> Responder<'a> for CORSPreflightResponder {
    fn respond_to(self, _: &Request) -> response::Result<'a> {
        let allowed_origins = self.allowed_origins.join(",");
        let max_age = self.max_age.to_string();
        Response::build()
            .status(Status::NoContent)
            .raw_header("Access-Control-Allow-Origin", allowed_origins)
            // TODO: Pass methods in programmatically.
            .raw_header("Access-Control-Allow-Methods", "OPTIONS, POST, PUT")
            .raw_header("Access-Control-Allow-Headers", "*")
            .raw_header("Access-Control-Max-Age", max_age)
            .header(Connection::keep_alive())
            .ok()
    }
}

#[options("/api/command")]
fn set_command_options() -> CORSPreflightResponder {
    CORSPreflightResponder {
        allowed_origins: vec!["*".to_owned()],
        max_age: 86400,
    }
}

#[put("/api/command", data = "<command>")]
fn set_command(db: State<Arc<Mutex<DB>>>, command: Json<Command>) -> Response {
    let mut db_ref = db.lock().unwrap();
    db_ref.command = Some(command.0);
    db_ref.refresh_requested = true;
    drop(db_ref);

    let mut response = Response::new();
    response.adjoin_raw_header("Access-Control-Allow-Origin", "*");
    response
}

struct CommandInvoker {
    db_ref: Arc<Mutex<DB>>,
}

impl CommandInvoker {
    fn new(db_ref: Arc<Mutex<DB>>) -> Self {
        Self {
            db_ref: db_ref.clone(),
        }
    }
}

impl Read for CommandInvoker {
    fn read(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let db_ref = (*self.db_ref).lock().unwrap();
        let db = db_ref.clone();
        drop(db_ref);

        return match (db.refresh_requested, db.command) {
            (false, _) => Ok(0),
            // If there's no command, there's nothing to do.
            (_, None) => Ok(0),
            // Otherwise, run the command and write its output to the buffer.
            (_, Some(command)) => {
                let mut db_ref = (*self.db_ref).lock().unwrap();
                db_ref.refresh_requested = false;
                drop(db_ref);

                let child = process::Command::new("bash")
                    .args(vec!["-c", &command.command])
                    .spawn()?;
                match child.stdout.as_mut() {
                    None => {
                        Err(io::Error::new(io::ErrorKind::Other, "Could not spawn the command."))
                    },
                    Some(stdout) => {
                        let data = Message(output).to_server_side_event().unwrap();
                        buf.write_all(&data)?;
                        Ok(data.len())
                    },
                }
            }
        };
    }
}

#[get("/api/command/output")]
fn stdout(db: State<Arc<Mutex<DB>>>) -> Response {
    let reader = BufReader::with_capacity(4294967296, CommandInvoker::new(db.clone()));
    let mut response = Response::new();
    response.adjoin_raw_header("Access-Control-Allow-Origin", "http://localhost:3000");
    response.adjoin_raw_header("Cache-Control", "no-cache");
    response.adjoin_raw_header("Content-Type", "text/event-stream");
    response.adjoin_raw_header("Expires", "0");
    response.set_streamed_body(reader);
    response
}

#[options("/api/join")]
fn join_options() -> CORSPreflightResponder {
    CORSPreflightResponder {
        allowed_origins: vec!["http://localhost:3000".to_owned()],
        max_age: 86400,
    }
}

#[post("/api/join")]
fn join(db: State<Arc<Mutex<DB>>>) -> Response {
    let mut db_ref = db.lock().unwrap();
    db_ref.refresh_requested = true;
    drop(db_ref);

    let mut response = Response::new();
    response.adjoin_raw_header("Access-Control-Allow-Origin", "*");
    response
}

fn main() {
    rocket::ignite()
        .manage(Arc::new(Mutex::new(DB {
            command: None,
            refresh_requested: true,
        })))
        .mount(
            "/",
            routes![
                join,
                join_options,
                set_command,
                set_command_options,
                stdout
            ],
        )
        .launch();
}
