#![feature(proc_macro_hygiene, decl_macro)]

#[macro_use] extern crate rocket;
extern crate serde;

use rocket::State;
use rocket::response::Stream;
use rocket_contrib::json::Json;
use serde::Deserialize;
use std::io::{self, Cursor, Read};
use std::process;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
    last_check: Option<Instant>,
}

impl CommandStream {
    fn new(db_ref: Arc<Mutex<DB>>) -> Self {
        Self {
            db_ref: db_ref.clone(),
            last_check: None,
        }
    }
}

impl Read for CommandStream {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let db_ref = (*self.db_ref).lock().unwrap();
        let db = db_ref.clone();
        drop(db_ref);

        return match (self.last_check, db.interval, db.command) {
            // If there's no command, there's nothing to do.
            (_, _, None) => Ok(0),
            // If there is no interval set, and we've already run the command, there's nothing to do.
            (Some(_), None, _) => Ok(0),
            // If we've checked before, but not enough time has elapsed yet, there's nothing to do.
            (Some(last_check), Some(interval), _) if (Instant::now() - last_check) < interval => Ok(0),
            // Otherwise, run the command and write its output to the buffer.
            (_, _, Some(command)) => {
                self.last_check = Some(Instant::now());
                let out = process::Command::new(command.binary).args(command.args).output()?;
                // TODO: Need a way to differentiate out and err when sending to the client.
                Cursor::new(out.stdout).read(buf)
            },
        };
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
    rocket::ignite().manage(Arc::new(Mutex::new(DB { command: None, interval: None }))).mount("/", routes![set_command, set_interval, stdout]).launch();
}
