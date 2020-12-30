/// Responsible for running commands, per-client, with retries.
/// Scaling to large numbers of clients is not an explicit goal of this architecture.
/// It is intended to robustly support multiple tabs open displaying shble output for a single user.
use std::collections::HashMap;
use std::io::{self, BufReader, Read};
use std::process::{Child, Command, ExitStatus};
use std::sync::mpsc::{self, Sender};
use std::thread;

use crate::parser;

enum ClientState {
    Canceled {
        command: String,
        child: Child,
    },
    Running {
        command: String,
        child: Child,
    },
    Failed(io::Error),
    Finished {
        command: String,
        stdout: Vec<Vec<String>>,
        stderr: String,
        status: ExitStatus,
    },
}

pub struct InvokeMessage {
    client_id: String,
    command: String,
}

pub struct CancelMessage {
    client_id: String,
}

pub struct Registry {
    // Keeps track of multiple client states.
    // A client can be a browser tab on the same, or a different machine.
    states: HashMap<String, ClientState>,
    invoke_sender_chan: Sender<InvokeMessage>,
    cancel_sender_chan: Sender<CancelMessage>,
}

impl Registry {
    pub fn new() -> Registry {
        let (invoke_sender_chan, invoke_receiver_chan) = mpsc::channel();
        let (cancel_sender_chan, cancel_receiver_chan) = mpsc::channel();
        let mut result = Registry {
            states: HashMap::new(),
            invoke_sender_chan,
            cancel_sender_chan,
        };
        let waiter_thread = thread::spawn(|| {
            loop {
                // Each loop, look for newly connected clients and run their command.
                // Do not block; move on immediately without waiting.
                for InvokeMessage { client_id, command } in invoke_receiver_chan.try_iter() {
                    // Running the command through `bash -c` allows the user to use environment variables, bash arg parsing, etc.
                    match Command::new("bash").args(vec!["-c", &command]).spawn() {
                        Err(error) => {
                            result.states.insert(client_id, ClientState::Failed(error));
                        }
                        Ok(child) => {
                            result
                                .states
                                .insert(client_id, ClientState::Running { command, child });
                        }
                    }
                }

                // Check for finished commands.
                // This should ensure that wait() is called for every child process.
                for (client_id, state) in result.states {
                    if let ClientState::Running { command, child } = state {
                        match child.try_wait() {
                            Err(error) => {
                                result.states.insert(client_id, ClientState::Failed(error));
                            }
                            Ok(None) => (),
                            Ok(Some(status)) => {
                                let stderr = match child.stderr {
                                    None => base64::encode(b""),
                                    Some(stderr) => base64::encode(stderr.into()),
                                };
                                let stdout = match child.stdout {
                                    None => Ok(vec![]),
                                    Some(stdout) => {
                                        let reader = BufReader::new(stdout);
                                        let mut bytes = vec![];
                                        match reader.read_to_end(&mut bytes) {
                                            Err(error) => Err(error),
                                            _ => parser::transform_2d(
                                                line_separators,
                                                line_regex,
                                                line_index_rules,
                                                row_separators,
                                                row_regex,
                                                row_index_rules,
                                                bytes,
                                            ),
                                        };
                                    }
                                };
                                result.states.insert(
                                    client_id,
                                    ClientState::Finished {
                                        command,
                                        status,
                                        stderr,
                                        stdout,
                                    },
                                );
                            }
                        };
                    }
                }
            }
        });
        result
    }

    pub fn invoke(&self, client_id: &str, command: &str) {}

    pub fn get(&self, client_id: &str) -> Option<Vec<Message>> {}
}
