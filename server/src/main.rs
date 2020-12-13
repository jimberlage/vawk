#![feature(proc_macro_hygiene, decl_macro)]

extern crate base64;
#[macro_use]
extern crate rocket;
extern crate serde;
extern crate serde_json;

use rocket::http::hyper::header::Connection;
use rocket::http::Status;
use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::State;
use rocket_contrib::json::Json;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize};
use std::io::{self, Cursor, Read};
use std::process::{self, Output};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    interval: Option<Duration>,
    last_read: Option<Instant>,
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
            .raw_header("Access-Control-Allow-Methods", "OPTIONS, PUT")
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
    let mut response = Response::new();
    response.adjoin_raw_header("Access-Control-Allow-Origin", "*");
    response
}

struct CommandStream {
    db_ref: Arc<Mutex<DB>>,
}

impl CommandStream {
    fn new(db_ref: Arc<Mutex<DB>>) -> Self {
        Self {
            db_ref: db_ref.clone(),
        }
    }
}

impl Read for CommandStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let db_ref = (*self.db_ref).lock().unwrap();
        let db = db_ref.clone();
        drop(db_ref);

        return match (db.last_read, db.interval, db.command) {
            // If there's no command, there's nothing to do.
            (_, _, None) => Ok(0),
            // If there is no interval set, and we've already run the command, there's nothing to do.
            (Some(_), None, _) => Ok(0),
            // If we've checked before, but not enough time has elapsed yet, there's nothing to do.
            (Some(last_read), Some(interval), _) if (Instant::now() - last_read) < interval => {
                Ok(0)
            }
            // Otherwise, run the command and write its output to the buffer.
            (_, _, Some(command)) => {
                let mut db_ref = (*self.db_ref).lock().unwrap();
                db_ref.last_read = Some(Instant::now());
                drop(db_ref);

                let output = process::Command::new("bash")
                    .args(vec!["-c", &command.command])
                    .output()?;
                Cursor::new(Message(output).to_server_side_event().unwrap()).read(buf)
            }
        };
    }
}

impl<'r> Responder<'r> for CommandStream {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        Response::build()
            .raw_header("Access-Control-Allow-Origin", "http://localhost:3000")
            .raw_header("Cache-Control", "no-cache")
            .raw_header("Content-Type", "text/event-stream")
            .raw_header("Expires", "0")
            .streamed_body(self)
            .ok()
    }
}

#[get("/api/command/stdout")]
fn stdout(db: State<Arc<Mutex<DB>>>) -> CommandStream {
    CommandStream::new(db.clone())
}

#[options("/api/interval")]
fn set_interval_options() -> CORSPreflightResponder {
    CORSPreflightResponder {
        allowed_origins: vec!["http://localhost:3000".to_owned()],
        max_age: 86400,
    }
}

#[put("/api/interval", data = "<interval>")]
fn set_interval(db: State<Arc<Mutex<DB>>>, interval: Json<Duration>) -> Response {
    let mut db_ref = db.lock().unwrap();
    db_ref.interval = Some(interval.0);
    let mut response = Response::new();
    response.adjoin_raw_header("Access-Control-Allow-Origin", "*");
    response
}

fn main() {
    rocket::ignite()
        .manage(Arc::new(Mutex::new(DB {
            command: None,
            interval: None,
            last_read: None,
        })))
        .mount(
            "/",
            routes![
                set_command,
                set_command_options,
                set_interval,
                set_interval_options,
                stdout
            ],
        )
        .launch();
}
