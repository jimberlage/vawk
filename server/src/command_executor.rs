/// Responsible for running commands, per-client, with retries.
/// Scaling to large numbers of clients is not an explicit goal of this architecture.
/// It is intended to robustly support multiple tabs open displaying shble output for a single user.
use crate::byte_trie::ByteTrie;
use crate::encoding;
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
use std::io::{self, BufReader, Read};
use std::num::Wrapping;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Instant;
use tokio::sync::mpsc;
use ulid::Ulid;

#[derive(Debug)]
enum CommandStatus {
    Idle,
    Canceled {
        id: usize,
        command: String,
    },
    Canceling {
        id: usize,
        child: Child,
        command: String,
    },
    CancellationFailed {
        id: usize,
        error: io::Error,
    },
    Running {
        id: usize,
        child: Child,
        command: String,
    },
    Failed {
        id: usize,
        error: io::Error,
    },
    Finished {
        id: usize,
        command: String,
        status: ExitStatus,
        stderr: Vec<u8>,
        stdout: Vec<u8>,
    },
}

struct ServerConnection {
    last_active: Instant,
    line_options: transformers::Options,
    receiver: Option<mpsc::Receiver<web::Bytes>>,
    row_options: transformers::Options,
    sender: mpsc::Sender<web::Bytes>,
    status: CommandStatus,
}

impl ServerConnection {
    fn new(sender: mpsc::Sender<web::Bytes>, receiver: mpsc::Receiver<web::Bytes>) -> Self {
        ServerConnection {
            last_active: Instant::now(),
            line_options: transformers::Options::default(),
            receiver: Some(receiver),
            row_options: transformers::Options::default(),
            sender,
            status: CommandStatus::Idle,
        }
    }
}

pub struct UnconnectedError {}

pub struct CommandExecutor {
    next_id: Wrapping<usize>,
    clients: HashMap<Ulid, ServerConnection>,
}

impl CommandExecutor {
    pub fn new() -> Self {
        CommandExecutor {
            next_id: Wrapping(0usize),
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

    fn listen(&mut self, client_id: &Ulid) -> Result<ClientConnection, UnconnectedError> {
        match self.clients.remove(client_id) {
            None => Err(UnconnectedError {}),
            Some(mut connection) => {
                match connection.receiver {
                    // TODO: There may be something better here, but forcing the client to reconnect seems reasonable.
                    None => Err(UnconnectedError {}),
                    Some(receiver) => {
                        connection.receiver = None;
                        self.clients.insert(*client_id, connection);
                        Ok(ClientConnection { receiver })
                    },
                }
            },
        }
    }

    fn process_output(&mut self, client_id: &Ulid) {
        match self.clients.get(client_id) {
            None => {
                log::error!("A client that no longer exists was asked for output: client_id: {}", client_id);
            },
            Some(connnection) => {
                if let CommandStatus::Finished {
                    id,
                    command: _,
                    ref status,
                    ref stderr,
                    ref stdout,
                } = connnection.status
                {
                    let transformed_stdout =
                        transformers::transform_2d(&connnection.line_options, &connnection.row_options, stdout);
                    match encoding::stdout_chunks(&transformed_stdout, id) {
                        Err(error) => {
                            log::error!("Failed to encode stdout: client_id: {}, error: {:#?}", client_id, error);
                        },
                        Ok(stdout_chunks) => {
                            match encoding::stderr_chunks(stderr, id) {
                                Err(error) => {
                                    log::error!("Failed to encode stderr: client_id: {}, error: {:#?}", client_id, error);
                                },
                                Ok(stderr_chunks) => {
                                    for chunk in stdout_chunks {
                                        if let Err(error) = connnection.sender.try_send(chunk) {
                                            log::error!("Failed to send a chunk of stdout, client disconnected or there is too much chatter: client_id: {}, error: {:#?}", client_id, error);
                                        }
                                    }
                                    for chunk in stderr_chunks {
                                        if let Err(error) = connnection.sender.try_send(chunk) {
                                            log::error!("Failed to send a chunk of stderr, client disconnected or there is too much chatter: client_id: {}, error: {:#?}", client_id, error);
                                        }
                                    }
                                    if let Err(error) = connnection.sender.try_send(encoding::status_message(status, id)) {
                                        log::error!("Failed to send an exit status, client disconnected or there is too much chatter: client_id: {}, error: {:#?}", client_id, error);
                                    }
                                },
                            }
                        },
                    }
                }
            }
        };
    }

    fn check_children(&mut self) -> Vec<Ulid> {
        let mut finished_children = vec![];
        for (client_id, connection) in self.clients.iter_mut() {
            match connection.status {
                CommandStatus::Canceling {
                    id,
                    ref mut child,
                    ref command,
                } => {
                    if let Err(error) = child.try_wait() {
                        match error.kind() {
                            io::ErrorKind::InvalidInput => (),
                            _ => {
                                connection.status = CommandStatus::Failed { id, error };
                                continue;
                            }
                        }
                    }

                    connection.status = CommandStatus::Canceled {
                        id,
                        command: command.into(),
                    };
                }
                CommandStatus::Running {
                    id,
                    ref mut child,
                    ref command,
                } => match child.try_wait() {
                    Err(error) => {
                        connection.status = CommandStatus::Failed { id, error };
                    }
                    Ok(None) => (),
                    Ok(Some(status)) => {
                        let stderr = match child.stderr {
                            None => vec![],
                            Some(ref mut stderr) => {
                                let mut reader = BufReader::new(stderr);
                                let mut bytes = vec![];
                                if let Err(error) = reader.read_to_end(&mut bytes) {
                                    connection.status = CommandStatus::Failed { id, error };
                                    continue;
                                };
                                bytes
                            }
                        };
                        match child.stdout {
                            None => {
                                connection.status = CommandStatus::Finished {
                                    id,
                                    command: command.into(),
                                    status,
                                    stderr,
                                    stdout: vec![],
                                };
                            }
                            Some(ref mut stdout) => {
                                let mut reader = BufReader::new(stdout);
                                let mut bytes = vec![];
                                match reader.read_to_end(&mut bytes) {
                                    Err(error) => {
                                        connection.status = CommandStatus::Failed { id, error };
                                    }
                                    _ => {
                                        connection.status = CommandStatus::Finished {
                                            id,
                                            command: command.into(),
                                            status,
                                            stderr,
                                            stdout: bytes,
                                        };
                                    }
                                };
                            }
                        };
                        finished_children.push(client_id.clone());
                    }
                },
                _ => (),
            };
        }
        finished_children
    }

    /// Set up a new connection for events related to command execution.
    fn connect(&mut self) -> Ulid {
        // Connect server to client with a multi-producer, single-consumer FIFO queue.
        // Tokio's implementation of mpsc queues is used to allow it to play nicely with the runtime.
        let (sender, receiver) = mpsc::channel(100);
        // Give the client a unique ID.
        // While this could technically block for a really long time, in practice the randomness of ULIDs should keep that from happening.
        let client_id = self.generate_client_id();
        // Allow the executor to send messages to the client.
        self.clients
            .insert(client_id, ServerConnection::new(sender, receiver));
        // And return the connection with its end of the queue, that can be used to stream messages.
        client_id
    }

    fn run(&mut self, client_id: &Ulid, command: String) -> Result<(), UnconnectedError> {
        let id = self.next_id.0;
        self.next_id = self.next_id + Wrapping(1usize);
        match self.clients.get_mut(client_id) {
            None => Err(UnconnectedError {}),
            Some(connection) => {
                connection.last_active = Instant::now();
                // Running the command through `bash -c` allows the user to use environment variables, bash arg parsing, etc.
                match Command::new("bash").args(vec!["-c", &command]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
                    Err(error) => {
                        connection.status = CommandStatus::Failed { id, error };
                    }
                    Ok(child) => {
                        connection.status = CommandStatus::Running {
                            id,
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
                if let CommandStatus::Running { id, mut child, command } = connection.status {
                    match child.kill() {
                        Err(error) if error.kind() != io::ErrorKind::InvalidInput => {
                            connection.status = CommandStatus::CancellationFailed { id, error };
                        }
                        _ => {
                            connection.status = CommandStatus::Canceling {
                                id,
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

    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.address().do_send(CheckChildren {});
    }
}

#[derive(Debug)]
pub struct ClientConnection {
    receiver: mpsc::Receiver<web::Bytes>,
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
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.receiver.poll_recv(cx).map(|bytes| bytes.map(|bytes| { Ok(bytes) }))
    }
}

#[derive(Deserialize)]
pub struct Listen {
    client_id: Ulid,
}

impl Message for Listen {
    type Result = Result<ClientConnection, UnconnectedError>;
}

impl Handler<Listen> for CommandExecutor {
    type Result = Result<ClientConnection, UnconnectedError>;

    fn handle(&mut self, msg: Listen, _ctx: &mut Self::Context) -> Self::Result {
        self.listen(&msg.client_id)
    }
}

// Connection

pub struct Connect {}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct ConnectResponse {
    client_id: Ulid,
}

impl<A, M> MessageResponse<A, M> for ConnectResponse
where
    A: Actor,
    M: Message<Result = ConnectResponse>,
{
    fn handle<R: ResponseChannel<M>>(self, _: &mut A::Context, tx: Option<R>) {
        if let Some(tx) = tx {
            tx.send(self);
        }
    }
}

impl Message for Connect {
    type Result = ConnectResponse;
}

impl Handler<Connect> for CommandExecutor {
    type Result = ConnectResponse;

    fn handle(&mut self, _msg: Connect, _ctx: &mut Self::Context) -> Self::Result {
        ConnectResponse { client_id: self.connect() }
    }
}

struct ProcessOutput {
    client_id: Ulid,
}

impl Message for ProcessOutput {
    type Result = ();
}

impl Handler<ProcessOutput> for CommandExecutor {
    type Result = ();

    fn handle(
        &mut self,
        msg: ProcessOutput,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
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

    fn handle(&mut self, msg: Run, _ctx: &mut Self::Context) -> Self::Result {
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

    fn handle(&mut self, msg: Cancel, _ctx: &mut Self::Context) -> Self::Result {
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
    ) -> Self::Result {
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
    ) -> Self::Result {
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
    ) -> Self::Result {
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
    ) -> Self::Result {
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
    ) -> Self::Result {
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
    ) -> Self::Result {
        self.set_row_separators(&msg.client_id, msg.separators)?;
        ctx.address().do_send(ProcessOutput {
            client_id: msg.client_id,
        });
        Ok(())
    }
}

pub struct CheckChildren {}

impl Message for CheckChildren {
    type Result = ();
}

impl Handler<CheckChildren> for CommandExecutor {
    type Result = ();

    fn handle(&mut self, _: CheckChildren, ctx: &mut Self::Context) -> Self::Result {
        let finished = self.check_children();
        for client_id in finished {
            ctx.address().do_send(ProcessOutput { client_id });
        }
        ctx.address().do_send(CheckChildren {});
    }
}