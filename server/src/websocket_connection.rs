/// This module provides an opinionated Websocket actor, suited to this project.
///
/// It provides:
/// - Heartbeat handling (clients are expected to ping every HEARTBEAT_INTERVAL and are disconnected if they stop responding)
/// - Continuation support (frames are collected and rolled into a single text or binary message, to reduce the number of handlers needed)
/// - Actor shutdown on close messages
///
/// For simplicity's sake, text messages are treated as binary.

use actix::prelude::*;
use actix_http::ws::{CloseCode, CloseReason, Item};
use actix_web_actors::ws;
use bytes::{Bytes, BytesMut};
use crate::parsers;
use crate::protos::definitions::{CompletedCommand, FromClient, FromClient_oneof_inner as FromClientInner, FromServer, FromServer_oneof_inner as FromServerInner, RunCommand, SetColumnIndexFilters, SetColumnRegexFilter, SetColumnSeparators, SetRowIndexFilters, SetRowRegexFilter, SetRowSeparators, UnexpectedError};
use crate::transformers;
use protobuf::{Message as ProtobufMessage, ProtobufError};
use std::cell::RefCell;
use std::fmt;
use std::io::{self, Read};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::rc::Rc;
use std::time::{Duration, Instant};

struct MessageParseError(ProtobufError);

impl fmt::Display for MessageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "An error occurred while parsing a message from the client:\n{}", self.0)
    }
}

struct EmptyMessageError;

impl fmt::Display for EmptyMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A message was expected but was not present.")
    }
}

#[derive(Debug)]
enum CancelError {
    KillError(io::Error),
}

impl fmt::Display for CancelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CancelError::KillError(error) => write!(f, "An error occurred while killing the current command process:\n{}", error),
        }
    }
}

#[derive(Debug)]
enum RunError {
    SpawnError(io::Error),
}

impl fmt::Display for RunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunError::SpawnError(error) => write!(f, "An error occurred while spawning the current command process:\n{}", error),
        }
    }
}

#[derive(Debug)]
enum WorkerError {
    WaitError(io::Error),
    ReadStdoutError(io::Error),
    ReadStderrError(io::Error),
    TransformStdoutError(io::Error),
    EncodeCommandError(ProtobufError),
}

impl fmt::Display for WorkerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkerError::WaitError(error) => write!(f, "An error occurred while calling wait() on the current command process:\n{}", error),
            WorkerError::ReadStdoutError(error) => write!(f, "An error occurred while reading stdout from the current command process:\n{}", error),
            WorkerError::ReadStderrError(error) => write!(f, "An error occurred while reading stderr from the current command process:\n{}", error),
            WorkerError::TransformStdoutError(error) => write!(f, "An error occurred while turning stdout for the current command process into CSV format:\n{}", error),
            WorkerError::EncodeCommandError(error) => write!(f, "An error occurred while encoding the current command output to be sent over the wire:\n{}", error),
        }
    }
}

#[derive(Debug, Clone)]
struct CancelingCommandStatus {
    child: Rc<RefCell<Child>>,
    command: String,
}

#[derive(Debug, Clone)]
struct FinishedCommandStatus {
    command: String,
    status: ExitStatus,
    stderr: Rc<Vec<u8>>,
    stdout: Rc<Vec<u8>>,
}

#[derive(Debug, Clone)]
struct RunningCommandStatus {
    child: Rc<RefCell<Child>>,
    command: String,
    stderr: Rc<RefCell<Vec<u8>>>,
    stdout: Rc<RefCell<Vec<u8>>>,
}

#[derive(Debug, Clone)]
enum CommandStatus {
    Canceled {
        command: String,
    },
    Canceling(CancelingCommandStatus),
    CancellationFailed,
    Failed,
    Finished(FinishedCommandStatus),
    Idle,
    Running(RunningCommandStatus),
}

/// How often heartbeat pings are sent.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout.
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct WebsocketConnection {
    column_options: transformers::Options,
    row_options: transformers::Options,
    command_status: CommandStatus,
    should_resend_csv: bool,
    last_seen_heartbeat: Instant,
    continuation_frame: Option<BytesMut>
}

const BUFFER_SIZE: usize = 5 * 1024;

fn read_and_extend<R: Read>(mut reader: R, result: &mut Vec<u8>) -> io::Result<bool> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let bytes_read = reader.read(&mut buffer)?;

    for i in 0..bytes_read {
        result.push(buffer[i]);
    }

    if bytes_read < BUFFER_SIZE {
        return Ok(true);
    }

    Ok(false)
}

impl WebsocketConnection {
    pub fn new(column_options: transformers::Options, row_options: transformers::Options) -> Self {
        Self {
            column_options,
            row_options,
            command_status: CommandStatus::Idle,
            should_resend_csv: false,
            last_seen_heartbeat: Instant::now(),
            continuation_frame: None,
        }
    }

    fn send_error<T: fmt::Display>(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>, error: T) {
        let mut error_response = FromServer::default();
        let mut error_wrapper = UnexpectedError::default();
        error_wrapper.set_description(format!("{}", error));
        error_response.inner = Some(FromServerInner::unexpected_error(error_wrapper));

        log::error!("{}", error);

        match error_response.write_to_bytes() {
            Ok(encoded_error_response) => ctx.binary(encoded_error_response),
            Err(error) => {
                log::error!("{}", error);
            }
        }
    }

    fn send_csvs(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>, status: ExitStatus, stdout: &Vec<u8>, stderr: &Vec<u8>) -> Result<(), WorkerError> {
        let transformed_stdout = transformers::transform_output(&self.column_options, &self.row_options, &stdout).map_err(|error| WorkerError::TransformStdoutError(error))?;
        
        let mut completed_command_response = FromServer::default();
        let mut completed_command_wrapper = CompletedCommand::default();
        completed_command_wrapper.set_was_successful(status.success());
        completed_command_wrapper.set_stdout(transformed_stdout);
        completed_command_wrapper.set_stderr(stderr.clone());
        completed_command_response.inner = Some(FromServerInner::completed_command(completed_command_wrapper));
        let encoded_completed_command_response = completed_command_response.write_to_bytes().map_err(|error| WorkerError::EncodeCommandError(error))?;

        ctx.binary(encoded_completed_command_response);

        self.should_resend_csv = false;

        Ok(())
    }

    fn on_canceling(&mut self, CancelingCommandStatus { child, command }: CancelingCommandStatus) -> Result<(), WorkerError> {
        if let Err(error) = (*child).borrow_mut().try_wait() {
            match error.kind() {
                io::ErrorKind::InvalidInput => (),
                _ => {
                    self.command_status = CommandStatus::Failed;
                    return Err(WorkerError::WaitError(error));
                }
            }
        }

        self.command_status = CommandStatus::Canceled {
            command: command.into(),
        };

        Ok(())
    }

    fn on_running(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>, RunningCommandStatus { child, command, stderr, stdout }: RunningCommandStatus) -> Result<(), WorkerError> {
        dbg!(command.clone());
        let mut finished_stderr = false;
        if let Some(stderr_handle) = (*child).borrow_mut().stderr.take() {
            finished_stderr = read_and_extend(stderr_handle, (*stderr).borrow_mut().as_mut()).map_err(|error| WorkerError::ReadStderrError(error))?;
        }

        let mut finished_stdout = false;
        if let Some(stdout_handle) = (*child).borrow_mut().stdout.take() {
            finished_stdout = read_and_extend(stdout_handle, (*stdout).borrow_mut().as_mut()).map_err(|error| WorkerError::ReadStdoutError(error))?;
        }

        let maybe_status = Some((*child).borrow_mut().wait().map_err(|error| WorkerError::WaitError(error))?);
        if let Some(status) = maybe_status {
            if finished_stderr && finished_stdout {
                self.command_status = CommandStatus::Finished(FinishedCommandStatus {
                    command: command.into(),
                    status,
                    stderr: Rc::new((*stderr).borrow().clone()),
                    stdout: Rc::new((*stdout).borrow().clone()),
                });
    
                self.send_csvs(ctx, status, &(*stdout).borrow(), &(*stderr).borrow())?;
            }
        }

        Ok(())
    }

    fn check_status(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>) {
        let result = match self.command_status.clone() {
            CommandStatus::Canceling(canceling_command_status) => self.on_canceling(canceling_command_status),
            CommandStatus::Running(running_command_status) => self.on_running(ctx, running_command_status),
            CommandStatus::Finished(finished_command_status) if self.should_resend_csv => {
                self.send_csvs(ctx, finished_command_status.status, &finished_command_status.stdout, &finished_command_status.stderr)
            },
            _ => Ok(()),
        };
        if let Err(error) = result {
            self.send_error(ctx, error);
        }
    }

    fn run(&mut self, command: RunCommand) -> Result<(), RunError> {
        let command_str = command.get_command();
        match Command::new("sh")
            .arg("-c")
            .arg(command_str)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
        {
            Ok(child) => {
                self.command_status = CommandStatus::Running(RunningCommandStatus {
                    command: command_str.to_owned(),
                    child: Rc::new(RefCell::new(child)),
                    stderr: Rc::new(RefCell::new(vec![])),
                    stdout: Rc::new(RefCell::new(vec![])),
                });
                Ok(())
            },
            Err(error) => {
                self.command_status = CommandStatus::Failed;
                Err(RunError::SpawnError(error))
            },
        }
    }

    fn cancel(&mut self) -> Result<(), CancelError> {
        let (maybe_new_command_status, result) = match &self.command_status {
            CommandStatus::Running(RunningCommandStatus { child, command, stderr: _, stdout: _ }) => {
                match (**child).borrow_mut().kill() {
                    // Already canceled.
                    Err(error) if error.kind() == io::ErrorKind::InvalidInput => (None, Ok(())),
                    Err(error) => (Some(CommandStatus::CancellationFailed), Err(CancelError::KillError(error))),
                    Ok(()) => {
                        (
                            Some(CommandStatus::Canceling(CancelingCommandStatus {
                                child: child.clone(),
                                command: command.clone(),
                            })),
                            Ok(())
                        )
                    },
                }
            },
            _ => (None, Ok(())),
        };

        if let Some(new_command_status) = maybe_new_command_status {
            self.command_status = new_command_status;
        }

        result
    }

    fn set_column_index_filters(&mut self, filters: SetColumnIndexFilters) -> Result<(), parsers::InvalidIndexFiltersError> {
        self.should_resend_csv = true;
        match parsers::parse_index_filters(filters.get_filters()) {
            Ok(parsed_filters) => {
                self.column_options.index_filters = Some(parsed_filters);
                Ok(())
            },
            Err(error) => {
                self.column_options.index_filters = None;
                Err(error)
            },
        }
    }

    fn set_column_regex_filter(&mut self, filter: SetColumnRegexFilter) -> Result<(), parsers::InvalidRegexFilterError> {
        self.should_resend_csv = true;
        match parsers::parse_regex_filter(filter.get_filter()) {
            Ok(parsed_filter) => {
                self.column_options.regex_filter = Some(parsed_filter);
                Ok(())
            },
            Err(error) => {
                self.column_options.regex_filter = None;
                Err(error)
            },
        }
    }

    fn set_column_separators(&mut self, separators: SetColumnSeparators) -> Result<(), parsers::InvalidFieldSeparatorError> {
        self.should_resend_csv = true;
        match parsers::parse_field_separators(separators.get_separators()) {
            Ok(parsed_separators) => {
                self.column_options.separators = Some(parsed_separators);
                Ok(())
            },
            Err(error) => {
                self.column_options.separators = None;
                Err(error)
            },
        }
    }

    fn set_row_index_filters(&mut self, filters: SetRowIndexFilters) -> Result<(), parsers::InvalidIndexFiltersError> {
        self.should_resend_csv = true;
        match parsers::parse_index_filters(filters.get_filters()) {
            Ok(parsed_filters) => {
                self.row_options.index_filters = Some(parsed_filters);
                Ok(())
            },
            Err(error) => {
                self.row_options.index_filters = None;
                Err(error)
            },
        }
    }

    fn set_row_regex_filter(&mut self, filter: SetRowRegexFilter) -> Result<(), parsers::InvalidRegexFilterError> {
        self.should_resend_csv = true;
        match parsers::parse_regex_filter(filter.get_filter()) {
            Ok(parsed_filter) => {
                self.row_options.regex_filter = Some(parsed_filter);
                Ok(())
            },
            Err(error) => {
                self.row_options.regex_filter = None;
                Err(error)
            },
        }
    }

    fn set_row_separators(&mut self, separators: SetRowSeparators) -> Result<(), parsers::InvalidFieldSeparatorError> {
        self.should_resend_csv = true;
        match parsers::parse_field_separators(separators.get_separators()) {
            Ok(parsed_separators) => {
                self.row_options.separators = Some(parsed_separators);
                Ok(())
            },
            Err(error) => {
                self.row_options.separators = None;
                Err(error)
            },
        }
    }

    fn handle_message(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>, data: Bytes) {
        match FromClient::parse_from_bytes(&data.to_vec()) {
            Ok(message) => {
                match message.inner {
                    Some(FromClientInner::cancel_command(_cancel_command)) => {
                        if let Err(error) = self.cancel() {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::run_command(run_command)) => {
                        if let Err(error) = self.run(run_command) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_column_index_filters(set_column_index_filters)) => {
                        if let Err(error) = self.set_column_index_filters(set_column_index_filters) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_column_regex_filter(set_column_regex_filter)) => {
                        if let Err(error) = self.set_column_regex_filter(set_column_regex_filter) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_column_separators(set_column_separators)) => {
                        if let Err(error) = self.set_column_separators(set_column_separators) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_row_index_filters(set_row_index_filters)) => {
                        if let Err(error) = self.set_row_index_filters(set_row_index_filters) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_row_regex_filter(set_row_regex_filter)) => {
                        if let Err(error) = self.set_row_regex_filter(set_row_regex_filter) {
                            self.send_error(ctx, error);
                        }
                    },
                    Some(FromClientInner::set_row_separators(set_row_separators)) => {
                        if let Err(error) = self.set_row_separators(set_row_separators) {
                            self.send_error(ctx, error);
                        }
                    },
                    None => {
                        self.send_error(ctx, EmptyMessageError);
                    }
                }
            }
            Err(error) => {
                self.send_error(ctx, MessageParseError(error));
            }
        }
    }

    fn set_first_frame_part(&mut self, data: Bytes) {
        let mut frame_data = BytesMut::with_capacity(2 * data.len());
        frame_data.extend(data.iter());
        self.continuation_frame = Some(frame_data);
    }

    fn set_frame_part(&mut self, additional_data: Bytes) {
        match &mut self.continuation_frame {
            // If the client messes up and sends an invalid continuation, treat it as binary to be safe.
            // We're choosing not to error here; it would provide more error handling complexity than I would like to take on at this time.
            None => self.set_first_frame_part(additional_data),
            Some(data) => {
                data.extend(additional_data.iter());
            }
        }
    }

    fn set_last_frame_part(&mut self, additional_data: Bytes) {
        match &mut self.continuation_frame {
            // If the client messes up and sends an invalid continuation, treat it as binary to be safe.
            // We're choosing not to error here; it would provide more error handling complexity than I would like to take on at this time.
            None => self.set_first_frame_part(additional_data),
            Some(data) => {
                data.extend(additional_data.iter());
            }
        }
    }

    fn send_full_continuation_frame(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>) {
        if let Some(ref data) = self.continuation_frame {
            let frozen_data = data.clone().freeze();
            self.handle_message(ctx, frozen_data);
        }
    }
}

impl Actor for WebsocketConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, ctx: &mut Self::Context) {
        // Begin watching for the client's heartbeat messages.
        ctx.run_interval(HEARTBEAT_INTERVAL, |connection, ctx| {
            // Have we timed out?  If so, close this connection.
            if Instant::now().duration_since(connection.last_seen_heartbeat) > CLIENT_TIMEOUT {
                ctx.stop();
                return;
            }

            ctx.ping(b"");
        });

        ctx.run_interval(Duration::from_millis(250), |connection, ctx| {
            connection.check_status(ctx);
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketConnection {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        match msg {
            Ok(ws::Message::Nop) => {},
            Ok(ws::Message::Continuation(Item::FirstText(data))) => {
                self.set_first_frame_part(data);
            },
            Ok(ws::Message::Continuation(Item::FirstBinary(data))) => {
                self.set_first_frame_part(data);
            },
            Ok(ws::Message::Continuation(Item::Continue(additional_data))) => {
                self.set_frame_part(additional_data);
            },
            Ok(ws::Message::Continuation(Item::Last(additional_data))) => {
                self.set_last_frame_part(additional_data);
                self.send_full_continuation_frame(ctx);
            },
            Ok(ws::Message::Ping(data)) => {
                self.last_seen_heartbeat = Instant::now();
                ctx.pong(&data);
            }
            Ok(ws::Message::Pong(_)) => {
                self.last_seen_heartbeat = Instant::now();
            }
            Ok(ws::Message::Text(text)) => {
                let mut data = BytesMut::new();
                data.extend(text.bytes());
                self.handle_message(ctx, data.freeze());
            },
            Ok(ws::Message::Binary(data)) => {
                self.handle_message(ctx, data);
            },
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
            }
            Err(error) => {
                log::error!("{}", error);
                ctx.close(Some(CloseReason::from(CloseCode::Error)));
                ctx.stop();
            },
        }
    }
}
