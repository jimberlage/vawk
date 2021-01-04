/// Responsible for running commands, per-client, with retries.
/// Scaling to large numbers of clients is not an explicit goal of this architecture.
/// It is intended to robustly support multiple tabs open displaying shble output for a single user.
use crate::byte_trie::ByteTrie;
use crate::parsers::IndexFilter;
use crate::transformers;
use actix::dev::{MessageResponse, ResponseChannel};
use actix::prelude::*;
use actix_web;
use actix_web::web;
use futures::Stream;
use regex::bytes::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::process::ExitStatus;
use std::time::Instant;
use tokio::io::{AsyncReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use ulid::Ulid;

enum Event {
    StderrChunk,
    StdoutChunk,
}

pub struct ClientConnection {
    client_id: Ulid,
    receiver: mpsc::Receiver<Event>,
}

impl ClientConnection {
    fn new(client_id: Ulid, receiver: mpsc::Receiver<Event>) -> Self {
        ClientConnection {
            client_id,
            receiver,
        }
    }
}

impl<A, M> MessageResponse<A, M> for ClientConnection
where
    A: Actor,
    M: Message<Result = ClientConnection>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Stream for ClientConnection {
    type Item = Result<web::Bytes, actix_web::Error>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        todo!()
    }
}

enum CommandStatus {
    Idle,
    Canceled {
        command: String,
    },
    Canceling {
        child: Child,
        command: String,
    },
    CancellationFailed {
        error: io::Error,
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
        status: ExitStatus,
        stderr: Vec<u8>,
        stdout: Vec<u8>,
    },
}

struct ServerConnection {
    last_active: Instant,
    line_options: transformers::Options,
    row_options: transformers::Options,
    sender: mpsc::Sender<Event>,
    status: CommandStatus,
}

impl ServerConnection {
    fn new(sender: mpsc::Sender<Event>) -> Self {
        ServerConnection {
            last_active: Instant::now(),
            line_options: transformers::Options::default(),
            row_options: transformers::Options::default(),
            sender,
            status: CommandStatus::Idle,
        }
    }
}

pub struct UnconnectedError {}

pub struct CommandExecutor {
    clients: HashMap<Ulid, ServerConnection>,
}

impl CommandExecutor {
    pub fn new() -> Self {
        CommandExecutor {
            clients: HashMap::new(),
        }
    }

    /// Generate a unique client ID.
    fn generate_client_id(&self) -> Ulid {
        let mut client_id = Ulid::new();
        while self.clients.contains_key(&client_id) {
            client_id = Ulid::new();
        }
        client_id
    }

    fn process_output(&self, client_id: &Ulid) -> Result<(), UnconnectedError> {
        match self.clients.get(client_id) {
            None => Err(UnconnectedError {}),
            Some(state) => {
                if let CommandStatus::Finished {
                    command: _,
                    status: _,
                    stderr: _,
                    ref stdout,
                } = state.status
                {
                    let _processed =
                        transformers::transform_2d(&state.line_options, &state.row_options, stdout);
                    // TODO: Break up into fixed size chunks and send back to the client connection.
                }
                Ok(())
            }
        }
    }

    async fn wait_for_output(&mut self) {
        for connection in self.clients.values_mut() {
            match connection.status {
                CommandStatus::Canceling {
                    ref mut child,
                    ref command,
                } => {
                    if let Err(error) = child.try_wait() {
                        match error.kind() {
                            io::ErrorKind::InvalidInput => (),
                            _ => {
                                connection.status = CommandStatus::Failed { error };
                                continue;
                            }
                        }
                    }

                    connection.status = CommandStatus::Canceled {
                        command: command.into(),
                    };
                }
                CommandStatus::Running {
                    ref mut child,
                    ref command,
                } => match child.try_wait() {
                    Err(error) => {
                        connection.status = CommandStatus::Failed { error };
                    }
                    Ok(None) => (),
                    Ok(Some(status)) => {
                        let stderr = match child.stderr {
                            None => vec![],
                            Some(ref mut stderr) => {
                                let mut reader = BufReader::new(stderr);
                                let mut bytes = vec![];
                                if let Err(error) = reader.read_to_end(&mut bytes).await {
                                    connection.status = CommandStatus::Failed { error };
                                    continue;
                                };
                                bytes
                            }
                        };
                        match child.stdout {
                            None => {
                                connection.status = CommandStatus::Finished {
                                    command: command.into(),
                                    status,
                                    stderr,
                                    stdout: vec![],
                                };
                            }
                            Some(ref mut stdout) => {
                                let mut reader = BufReader::new(stdout);
                                let mut bytes = vec![];
                                match reader.read_to_end(&mut bytes).await {
                                    Err(error) => {
                                        connection.status = CommandStatus::Failed { error };
                                    }
                                    _ => {
                                        connection.status = CommandStatus::Finished {
                                            command: command.into(),
                                            status,
                                            stderr,
                                            stdout: bytes,
                                        };
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

    /// Set up a new connection for events related to command execution.
    fn connect(&mut self) -> ClientConnection {
        // Connect server to client with a multi-producer, single-consumer FIFO queue.
        // Tokio's implementation of mpsc queues is used to allow it to play nicely with the runtime.
        let (sender, receiver) = mpsc::channel(100);
        // Give the client a unique ID.
        // While this could technically block for a really long time, in practice the randomness of ULIDs should keep that from happening.
        let client_id = self.generate_client_id();
        // Allow the executor to send messages to the client.
        self.clients
            .insert(client_id, ServerConnection::new(sender));
        // And return the connection with its end of the queue, that can be used to stream messages.
        ClientConnection::new(client_id, receiver)
    }

    fn run(&mut self, client_id: &Ulid, command: String) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                // Running the command through `bash -c` allows the user to use environment variables, bash arg parsing, etc.
                match Command::new("bash").args(vec!["-c", &command]).spawn() {
                    Err(error) => {
                        connection.status = CommandStatus::Failed { error };
                    }
                    Ok(child) => {
                        connection.status = CommandStatus::Running {
                            command,
                            child,
                        };
                    }
                };
                Ok(())
            }
        }
    }

    fn cancel(&mut self, client_id: &Ulid) -> Result<(), UnconnectedError> {
        match self.clients.remove(client_id) {
            None => Err(UnconnectedError {}),
            Some(mut connection) => {
                connection.last_active = Instant::now();
                if let CommandStatus::Running { mut child, command } = connection.status {
                    match child.start_kill() {
                        Err(error) if error.kind() != io::ErrorKind::InvalidInput => {
                            connection.status = CommandStatus::CancellationFailed { error };
                        }
                        _ => {
                            connection.status = CommandStatus::Canceling {
                                child,
                                command: command.clone(),
                            };
                        }
                    }
                }
                self.clients.insert(*client_id, connection);
                Ok(())
            }
        }
    }

    fn set_line_index_filters(
        &mut self,
        client_id: &Ulid,
        filters: Option<Vec<IndexFilter>>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.line_options.index_filters = filters;
                Ok(())
            }
        }
    }

    fn set_line_regex_filter(
        &mut self,
        client_id: &Ulid,
        filter: Option<Regex>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.line_options.regex_filter = filter;
                Ok(())
            }
        }
    }

    fn set_line_separators(
        &mut self,
        client_id: &Ulid,
        separators: Option<ByteTrie>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.line_options.separators = separators;
                Ok(())
            }
        }
    }

    fn set_row_index_filters(
        &mut self,
        client_id: &Ulid,
        filters: Option<Vec<IndexFilter>>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.row_options.index_filters = filters;
                Ok(())
            }
        }
    }

    fn set_row_regex_filter(
        &mut self,
        client_id: &Ulid,
        filter: Option<Regex>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.row_options.regex_filter = filter;
                Ok(())
            }
        }
    }

    fn set_row_separators(
        &mut self,
        client_id: &Ulid,
        separators: Option<ByteTrie>,
    ) -> Result<(), UnconnectedError> {
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                connection.row_options.separators = separators;
                Ok(())
            }
        }
    }
}

impl Actor for CommandExecutor {
    type Context = Context<Self>;
}

// Connection

pub struct Connect {}

impl Message for Connect {
    type Result = ClientConnection;
}

impl Handler<Connect> for CommandExecutor {
    type Result = ClientConnection;

    fn handle(&mut self, _msg: Connect, _ctx: &mut Self::Context) -> ClientConnection {
        self.connect()
    }
}

struct ProcessOutput {
    client_id: Ulid,
}

impl Message for ProcessOutput {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<ProcessOutput> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: ProcessOutput,
        _ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.process_output(&msg.client_id)
    }
}

// Run commands

#[derive(Deserialize, Serialize)]
pub struct Run {
    client_id: Ulid,
    command: String,
}

impl Message for Run {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<Run> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(&mut self, msg: Run, _ctx: &mut Self::Context) -> Result<(), UnconnectedError> {
        self.run(&msg.client_id, msg.command)
    }
}

// Cancel a running command

#[derive(Deserialize, Serialize)]
pub struct Cancel {
    client_id: Ulid,
}

impl Message for Cancel {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<Cancel> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(&mut self, msg: Cancel, _ctx: &mut Self::Context) -> Result<(), UnconnectedError> {
        self.cancel(&msg.client_id)
    }
}

// Setters for settings

pub struct SetLineIndexFilters {
    pub client_id: Ulid,
    pub filters: Option<Vec<IndexFilter>>,
}

impl Message for SetLineIndexFilters {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetLineIndexFilters> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetLineIndexFilters,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_line_index_filters(&msg.client_id, msg.filters)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct SetLineRegexFilter {
    pub client_id: Ulid,
    pub filter: Option<Regex>,
}

impl Message for SetLineRegexFilter {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetLineRegexFilter> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetLineRegexFilter,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_line_regex_filter(&msg.client_id, msg.filter)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct SetLineSeparators {
    pub client_id: Ulid,
    pub separators: Option<ByteTrie>,
}

impl Message for SetLineSeparators {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetLineSeparators> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetLineSeparators,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_line_separators(&msg.client_id, msg.separators)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct SetRowIndexFilters {
    pub client_id: Ulid,
    pub filters: Option<Vec<IndexFilter>>,
}

impl Message for SetRowIndexFilters {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetRowIndexFilters> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetRowIndexFilters,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_row_index_filters(&msg.client_id, msg.filters)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct SetRowRegexFilter {
    pub client_id: Ulid,
    pub filter: Option<Regex>,
}

impl Message for SetRowRegexFilter {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetRowRegexFilter> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetRowRegexFilter,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_row_regex_filter(&msg.client_id, msg.filter)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct SetRowSeparators {
    pub client_id: Ulid,
    pub separators: Option<ByteTrie>,
}

impl Message for SetRowSeparators {
    type Result = Result<(), UnconnectedError>;
}

impl Handler<SetRowSeparators> for CommandExecutor {
    type Result = Result<(), UnconnectedError>;

    fn handle(
        &mut self,
        msg: SetRowSeparators,
        ctx: &mut Self::Context,
    ) -> Result<(), UnconnectedError> {
        self.set_row_separators(&msg.client_id, msg.separators)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}