#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate serde;

use rocket::{State, response::Stream};
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::{io::{self, Cursor, Read}, process, sync::Arc, sync::Mutex, time::Duration};

#[derive(Clone, Deserialize)]
struct Command {
    binary: String,
    args: Vec<String>,
}

#[derive(Clone)]
struct DB {
    command: Option<Command>,
    interval: Option<Duration>,
}

#[post("/api/command", data = "<command>")]
fn set_command(db: State<Arc<Mutex<DB>>>, command: Json<Command>) {
    let mut db_ref = db.lock().unwrap();
    db_ref.command = Some(command.0);
}

struct CommandStream {
    db_ref: Arc<Mutex<DB>>,
    stdout: String,
}

impl CommandStream {
    fn new(db_ref: Arc<Mutex<DB>>) -> Self {
        Self {
            db_ref: db_ref.clone(),
            stdout: String::new(),
        }
    }
}

impl Read for CommandStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let db_ref = (*self.db_ref).lock().unwrap();
        let db = db_ref.clone();
        drop(db_ref);

        match db.command {
            None => Ok(0),
            Some(command) => {
                // TODO: Include intervals here.
                let out = process::Command::new(command.binary).args(command.args).output()?;
                // TODO: Need a way to differentiate out and err when sending to the client.
                Cursor::new(out.stdout).read(buf)
            }
        }
    }
}

#[get("/api/command/stdout")]
fn stdout(db: State<Arc<Mutex<DB>>>) -> Stream<CommandStream> {
    Stream::from(CommandStream::new(db.clone()))
}

#[post("/api/interval", data = "<interval>")]
fn set_interval(db: State<Arc<Mutex<DB>>>, interval: Json<Duration>) {
    let mut db_ref = db.lock().unwrap();
    db_ref.interval = Some(interval.0);
}

fn main() {
    rocket::ignite().manage(Mutex::new(DB { command: None, interval: None })).mount("/", routes![set_command, set_interval, stdout]).launch();
}
