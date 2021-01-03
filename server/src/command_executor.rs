/// Responsible for running commands, per-client, with retries.
/// Scaling to large numbers of clients is not an explicit goal of this architecture.
/// It is intended to robustly support multiple tabs open displaying shble output for a single user.
use crate::byte_trie::ByteTrie;
use crate::parsers::IndexFilter;
use crate::transformers;
use regex::bytes::Regex;
use std::collections::HashMap;
use std::io::{self, BufReader, Read};
use std::process::{Child, Command, ExitStatus};
use std::sync::mpsc::{self, Sender};
use std::thread;
use std::time::Instant;

enum ClientStateStatus {
    Idle,
    Canceled {
        command: String,
    },
    Canceling {
        child: Child,
        command: String,
    },
    Running {
        child: Child,
        command: String,
    },
    Failed {
        error: io::Error,
    },
    Finished {
        command: String,
        raw_stdout: Vec<u8>,
        status: ExitStatus,
        stderr: String,
        stdout: Vec<Vec<String>>,
    },
}

struct ClientState {
    last_active: Instant,
    line_options: transformers::Options,
    row_options: transformers::Options,
    status: ClientStateStatus,
}

enum InputMessage {
    Cancel {
        client_id: String,
    },
    Run {
        client_id: String,
        command: String,
    },
    SetLineIndexFilters {
        client_id: String,
        index_filters: Option<Vec<IndexFilter>>,
    },
    SetLineRegexFilter {
        client_id: String,
        regex_filter: Option<Regex>,
    },
    SetLineSeparator {
        client_id: String,
        separator: Option<ByteTrie>,
    },
    SetRowIndexFilters {
        client_id: String,
        index_filters: Option<Vec<IndexFilter>>,
    },
    SetRowRegexFilter {
        client_id: String,
        regex_filter: Option<Regex>,
    },
    SetRowSeparator {
        client_id: String,
        separator: Option<ByteTrie>,
    },
}

pub struct Executor {
    // Keeps track of multiple client states.
    // A client can be a browser tab on the same, or a different machine.
    states: HashMap<String, ClientState>,
    sender_chan: Sender<InputMessage>,
}

impl Executor {
    fn update_status(&mut self, client_id: String, status: ClientStateStatus) {
        match self.states.get_mut(&client_id) {
            None => {
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: transformers::Options::default(),
                        row_options: transformers::Options::default(),
                        status,
                    },
                );
            }
            Some(state) => {
                state.last_active = Instant::now();
                state.status = status;
            }
        }
    }

    fn handle_cancel(&mut self, client_id: String) {
        if let Some(state) = self.states.get_mut(&client_id) {
            if let ClientStateStatus::Running { child, command: _ } = state.status {
                if let Err(error) = child.kill() {
                    self.update_status(client_id, ClientStateStatus::Failed { error });
                }
            }
        }
    }

    fn handle_run(&mut self, client_id: String, command: String) {
        // Running the command through `bash -c` allows the user to use environment variables, bash arg parsing, etc.
        match Command::new("bash").args(vec!["-c", &command]).spawn() {
            Err(error) => {
                self.update_status(client_id, ClientStateStatus::Failed { error });
            }
            Ok(child) => {
                self.update_status(client_id, ClientStateStatus::Running { command, child });
            }
        }
    }

    fn handle_set_line_index_filters(
        &mut self,
        client_id: String,
        filters: Option<Vec<IndexFilter>>,
    ) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut line_options = transformers::Options::default();
                line_options.index_filters = filters;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: line_options,
                        row_options: transformers::Options::default(),
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.line_options.index_filters = filters;
            }
        }
    }

    fn handle_set_line_regex_filter(&mut self, client_id: String, filters: Option<Regex>) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut line_options = transformers::Options::default();
                line_options.regex_filter = filters;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: line_options,
                        row_options: transformers::Options::default(),
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.line_options.regex_filter = filters;
            }
        }
    }

    fn handle_set_line_separator(&mut self, client_id: String, separator: Option<ByteTrie>) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut line_options = transformers::Options::default();
                line_options.separators = separator;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: line_options,
                        row_options: transformers::Options::default(),
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.line_options.separators = separator;
            }
        }
    }

    fn handle_set_row_index_filters(
        &mut self,
        client_id: String,
        filters: Option<Vec<IndexFilter>>,
    ) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut row_options = transformers::Options::default();
                row_options.index_filters = filters;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: transformers::Options::default(),
                        row_options: row_options,
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.row_options.index_filters = filters;
            }
        }
    }

    fn handle_set_row_regex_filter(&mut self, client_id: String, filters: Option<Regex>) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut row_options = transformers::Options::default();
                row_options.regex_filter = filters;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: transformers::Options::default(),
                        row_options: row_options,
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.row_options.regex_filter = filters;
            }
        }
    }

    fn handle_set_row_separator(&mut self, client_id: String, separator: Option<ByteTrie>) {
        match self.states.get_mut(&client_id) {
            None => {
                let mut row_options = transformers::Options::default();
                row_options.separators = separator;
                self.states.insert(
                    client_id,
                    ClientState {
                        last_active: Instant::now(),
                        line_options: transformers::Options::default(),
                        row_options: row_options,
                        status: ClientStateStatus::Idle,
                    },
                );
            }
            Some(state) => {
                state.row_options.separators = separator;
            }
        }
    }

    fn try_wait_all(&mut self) {
        for (client_id, state) in self.states {
            match state.status {
                ClientStateStatus::Canceling { child, command } => {
                    if let Err(error) = child.try_wait() {
                        match error.kind() {
                            io::ErrorKind::InvalidInput => (),
                            _ => {
                                self.update_status(client_id, ClientStateStatus::Failed { error });
                                continue;
                            }
                        }
                    }

                    self.update_status(client_id, ClientStateStatus::Canceled { command });
                }
                ClientStateStatus::Running { child, command } => match child.try_wait() {
                    Err(error) => {
                        self.update_status(client_id, ClientStateStatus::Failed { error })
                    }
                    Ok(None) => (),
                    Ok(Some(status)) => {
                        let stderr = match child.stderr {
                            None => base64::encode(b""),
                            Some(stderr) => {
                                let reader = BufReader::new(stderr);
                                let mut bytes = vec![];
                                match reader.read_to_end(&mut bytes) {
                                    Err(error) => {
                                        self.update_status(
                                            client_id,
                                            ClientStateStatus::Failed { error },
                                        );
                                        continue;
                                    }
                                    _ => base64::encode(bytes),
                                }
                            }
                        };
                        match child.stdout {
                            None => {
                                self.update_status(
                                    client_id,
                                    ClientStateStatus::Finished {
                                        command,
                                        raw_stdout: vec![],
                                        status,
                                        stderr,
                                        stdout: vec![],
                                    },
                                );
                            }
                            Some(stdout) => {
                                let reader = BufReader::new(stdout);
                                let mut bytes = vec![];
                                match reader.read_to_end(&mut bytes) {
                                    Err(error) => {
                                        self.update_status(
                                            client_id,
                                            ClientStateStatus::Failed { error },
                                        );
                                    }
                                    _ => {
                                        self.update_status(
                                            client_id,
                                            ClientStateStatus::Finished {
                                                command,
                                                raw_stdout: bytes,
                                                status,
                                                stderr,
                                                stdout: transformers::encode_2d(
                                                    transformers::transform_2d(
                                                        &state.line_options,
                                                        &state.row_options,
                                                        bytes,
                                                    ),
                                                ),
                                            },
                                        );
                                    }
                                };
                            }
                        };
                    }
                },
                _ => (),
            };
        }
    }

    pub fn new() -> Executor {
        let (sender_chan, receiver_chan) = mpsc::channel();
        let mut result = Executor {
            states: HashMap::new(),
            sender_chan,
        };
        let waiter_thread = thread::spawn(move || {
            // No operation in this loop should block.
            loop {
                // Change the executor (run commands, set filters, etc.) according to user input.
                for message in receiver_chan.try_iter() {
                    match message {
                        InputMessage::Cancel { client_id } => result.handle_cancel(client_id),
                        InputMessage::Run { client_id, command } => {
                            result.handle_run(client_id, command)
                        }
                        InputMessage::SetLineIndexFilters {
                            client_id,
                            index_filters,
                        } => result.handle_set_line_index_filters(client_id, index_filters),
                        InputMessage::SetLineRegexFilter {
                            client_id,
                            regex_filter,
                        } => result.handle_set_line_regex_filter(client_id, regex_filter),
                        InputMessage::SetLineSeparator {
                            client_id,
                            separator,
                        } => result.handle_set_line_separator(client_id, separator),
                        InputMessage::SetRowIndexFilters {
                            client_id,
                            index_filters,
                        } => result.handle_set_row_index_filters(client_id, index_filters),
                        InputMessage::SetRowRegexFilter {
                            client_id,
                            regex_filter,
                        } => result.handle_set_row_regex_filter(client_id, regex_filter),
                        InputMessage::SetRowSeparator {
                            client_id,
                            separator,
                        } => result.handle_set_row_separator(client_id, separator),
                    }
                }

                // Check for finished commands.
                // This should ensure that wait() is called for every child process.
                result.try_wait_all();
            }
        });
        result
    }

    pub fn run(&self, client_id: &str, command: &str) {
        self.sender_chan
            .send(InputMessage::Run {
                client_id: client_id.into(),
                command: command.into(),
            })
            .unwrap();
    }

    pub fn cancel(&self, client_id: &str) {
        self.sender_chan
            .send(InputMessage::Cancel {
                client_id: client_id.into(),
            })
            .unwrap();
    }
}
