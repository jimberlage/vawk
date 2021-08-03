/// This module provides an opinionated Websocket actor, suited to this project.
///
/// It provides:
/// - Heartbeat handling (clients are expected to ping every HEARTBEAT_INTERVAL and are disconnected if they stop responding)
/// - Continuation support (frames are collected and rolled into a single text or binary message, to reduce the number of handlers needed)
/// - Actor shutdown on close messages
///
/// For simplicity's sake, text messages are treated as binary.
use crate::parsers;
use crate::protos::definitions::{
    Combination_oneof_inner as CombinationInner, FromClient,
    FromClient_oneof_inner as FromClientInner, FromServer,
    FromServer_oneof_inner as FromServerInner, Initialize, SetColumnFilterCombination,
    SetColumnIndexFilters, SetColumnRegexFilter, SetColumnRegexSeparator, SetColumnSeparators,
    SetRowFilterCombination, SetRowIndexFilters, SetRowRegexFilter, SetRowRegexSeparator,
    SetRowSeparators, UnexpectedError,
};
use crate::transformers;

use actix::prelude::*;
use actix_http::ws::{CloseCode, CloseReason, Item};
use actix_web_actors::ws;
use bytes::{Bytes, BytesMut};
use protobuf::{Message as ProtobufMessage, ProtobufError};
use std::fmt;
use std::io;
use std::time::{Duration, Instant};

struct MessageParseError(ProtobufError);

impl fmt::Display for MessageParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "An error occurred while parsing a message from the client:\n{}",
            self.0
        )
    }
}

enum InitializeError {
    InvalidRowFieldSeparatorError(parsers::InvalidFieldSeparatorError),
    InvalidRowIndexFiltersError(parsers::InvalidIndexFiltersError),
    InvalidRowRegexFilterError(parsers::InvalidRegexError),
    InvalidRowRegexSeparatorError(parsers::InvalidRegexError),
    InvalidColumnFieldSeparatorError(parsers::InvalidFieldSeparatorError),
    InvalidColumnIndexFiltersError(parsers::InvalidIndexFiltersError),
    InvalidColumnRegexFilterError(parsers::InvalidRegexError),
    InvalidColumnRegexSeparatorError(parsers::InvalidRegexError),
}

impl fmt::Display for InitializeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InitializeError::InvalidRowFieldSeparatorError(error) => {
                write!(f, "A row separator is invalid:\n{}", error)
            }
            InitializeError::InvalidRowIndexFiltersError(error) => {
                write!(f, "The row index filters are invalid:\n{}", error)
            }
            InitializeError::InvalidRowRegexFilterError(error) => {
                write!(f, "The row regex filter is invalid:\n{}", error)
            }
            InitializeError::InvalidRowRegexSeparatorError(error) => {
                write!(f, "The row regex separator is invalid:\n{}", error)
            }
            InitializeError::InvalidColumnFieldSeparatorError(error) => {
                write!(f, "A column separator is invalid:\n{}", error)
            }
            InitializeError::InvalidColumnIndexFiltersError(error) => {
                write!(f, "The column index filters are invalid:\n{}", error)
            }
            InitializeError::InvalidColumnRegexFilterError(error) => {
                write!(f, "The column regex filter is invalid:\n{}", error)
            }
            InitializeError::InvalidColumnRegexSeparatorError(error) => {
                write!(f, "The column regex separator is invalid:\n{}", error)
            }
        }
    }
}

struct EmptyMessageError;

impl fmt::Display for EmptyMessageError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "A message was expected but was not present.")
    }
}

#[derive(Debug)]
enum SendCSVError {
    TransformError(io::Error),
    EncodeCommandError(ProtobufError),
}

impl fmt::Display for SendCSVError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TransformError(error) => write!(f, "An error occurred while transforming command output:\n{}", error),
            Self::EncodeCommandError(error) => write!(f, "An error occurred while encoding the current command output to be sent over the wire:\n{}", error),
        }
    }
}

/// How often heartbeat pings are sent.
pub const HEARTBEAT_INTERVAL: Duration = Duration::from_millis(100);
/// How long before lack of client response causes a timeout.
pub const CLIENT_TIMEOUT: Duration = Duration::from_millis(500);

pub struct WebsocketConnection {
    stdin: Vec<u8>,
    column_options: transformers::Options,
    row_options: transformers::Options,
    last_seen_heartbeat: Instant,
    continuation_frame: Option<BytesMut>,
}

impl WebsocketConnection {
    pub fn new(
        stdin: Vec<u8>,
        column_options: transformers::Options,
        row_options: transformers::Options,
    ) -> Self {
        Self {
            stdin,
            column_options,
            row_options,
            last_seen_heartbeat: Instant::now(),
            continuation_frame: None,
        }
    }

    fn send_error<T: fmt::Display>(
        &mut self,
        ctx: &mut ws::WebsocketContext<WebsocketConnection>,
        error: T,
    ) {
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

    fn send_csvs(
        &mut self,
        ctx: &mut ws::WebsocketContext<WebsocketConnection>,
    ) -> Result<(), SendCSVError> {
        let transformed =
            transformers::transform_output(&self.column_options, &self.row_options, &self.stdin)
                .map_err(|error| SendCSVError::TransformError(error))?;

        let mut output_response = FromServer::default();
        output_response.inner = Some(FromServerInner::output(transformed));
        let encoded_output_response = output_response
            .write_to_bytes()
            .map_err(|error| SendCSVError::EncodeCommandError(error))?;

        ctx.binary(encoded_output_response);

        Ok(())
    }

    fn initialize(&mut self, initial_values: Initialize) -> Result<(), InitializeError> {
        self.row_options.separators = Some(
            parsers::parse_field_separators(initial_values.get_row_separators())
                .map_err(|error| InitializeError::InvalidRowFieldSeparatorError(error))?,
        );
        if initial_values.get_row_regex_separator() != "" {
            self.row_options.regex_separator = Some(
                parsers::parse_regex(initial_values.get_row_regex_separator())
                    .map_err(|error| InitializeError::InvalidRowRegexSeparatorError(error))?,
            );
        }
        if initial_values.get_row_index_filters() != "" {
            self.row_options.index_filters = Some(
                parsers::parse_index_filters(initial_values.get_row_index_filters())
                    .map_err(|error| InitializeError::InvalidRowIndexFiltersError(error))?,
            );
        }
        if initial_values.get_row_regex_filter() != "" {
            self.row_options.regex_filter = Some(
                parsers::parse_regex(initial_values.get_row_regex_filter())
                    .map_err(|error| InitializeError::InvalidRowRegexFilterError(error))?,
            );
        }
        self.row_options.filters_combination =
            match initial_values.get_row_filter_combination().inner {
                Some(CombinationInner::and(_)) => Some(transformers::Combination::And),
                Some(CombinationInner::or(_)) => Some(transformers::Combination::Or),
                None => None,
            };
        self.column_options.separators = Some(
            parsers::parse_field_separators(initial_values.get_column_separators())
                .map_err(|error| InitializeError::InvalidColumnFieldSeparatorError(error))?,
        );
        if initial_values.get_column_regex_separator() != "" {
            self.column_options.regex_separator = Some(
                parsers::parse_regex(initial_values.get_column_regex_separator())
                    .map_err(|error| InitializeError::InvalidColumnRegexSeparatorError(error))?,
            );
        }
        if initial_values.get_column_index_filters() != "" {
            self.column_options.index_filters = Some(
                parsers::parse_index_filters(initial_values.get_column_index_filters())
                    .map_err(|error| InitializeError::InvalidColumnIndexFiltersError(error))?,
            );
        }
        if initial_values.get_column_regex_filter() != "" {
            self.column_options.regex_filter = Some(
                parsers::parse_regex(initial_values.get_column_regex_filter())
                    .map_err(|error| InitializeError::InvalidColumnRegexFilterError(error))?,
            );
        }
        self.column_options.filters_combination =
            match initial_values.get_column_filter_combination().inner {
                Some(CombinationInner::and(_)) => Some(transformers::Combination::And),
                Some(CombinationInner::or(_)) => Some(transformers::Combination::Or),
                None => None,
            };

        Ok(())
    }

    fn set_column_index_filters(
        &mut self,
        filters: SetColumnIndexFilters,
    ) -> Result<(), parsers::InvalidIndexFiltersError> {
        if filters.get_filters() == "" {
            self.column_options.index_filters = None;
            return Ok(());
        }

        match parsers::parse_index_filters(filters.get_filters()) {
            Ok(parsed_filters) => {
                self.column_options.index_filters = Some(parsed_filters);
                Ok(())
            }
            Err(error) => {
                self.column_options.index_filters = None;
                Err(error)
            }
        }
    }

    fn set_column_regex_filter(
        &mut self,
        filter: SetColumnRegexFilter,
    ) -> Result<(), parsers::InvalidRegexError> {
        if filter.get_filter() == "" {
            self.column_options.regex_filter = None;
            return Ok(());
        }

        match parsers::parse_regex(filter.get_filter()) {
            Ok(parsed_filter) => {
                self.column_options.regex_filter = Some(parsed_filter);
                Ok(())
            }
            Err(error) => {
                self.column_options.regex_filter = None;
                Err(error)
            }
        }
    }

    fn set_column_filter_combination(&mut self, combination: SetColumnFilterCombination) {
        self.column_options.filters_combination = match combination.get_combination().inner {
            Some(CombinationInner::and(_)) => Some(transformers::Combination::And),
            Some(CombinationInner::or(_)) => Some(transformers::Combination::Or),
            None => None,
        }
    }

    fn set_column_separators(
        &mut self,
        separators: SetColumnSeparators,
    ) -> Result<(), parsers::InvalidFieldSeparatorError> {
        match parsers::parse_field_separators(separators.get_separators()) {
            Ok(parsed_separators) => {
                self.column_options.separators = Some(parsed_separators);
                Ok(())
            }
            Err(error) => {
                self.column_options.separators = None;
                Err(error)
            }
        }
    }

    fn set_column_regex_separator(
        &mut self,
        separator: SetColumnRegexSeparator,
    ) -> Result<(), parsers::InvalidRegexError> {
        if separator.get_separator() == "" {
            self.column_options.regex_separator = None;
            return Ok(());
        }

        match parsers::parse_regex(separator.get_separator()) {
            Ok(parsed_separator) => {
                self.column_options.regex_separator = Some(parsed_separator);
                Ok(())
            }
            Err(error) => {
                self.column_options.regex_separator = None;
                Err(error)
            }
        }
    }

    fn set_row_index_filters(
        &mut self,
        filters: SetRowIndexFilters,
    ) -> Result<(), parsers::InvalidIndexFiltersError> {
        if filters.get_filters() == "" {
            self.row_options.index_filters = None;
            return Ok(());
        }

        match parsers::parse_index_filters(filters.get_filters()) {
            Ok(parsed_filters) => {
                self.row_options.index_filters = Some(parsed_filters);
                Ok(())
            }
            Err(error) => {
                self.row_options.index_filters = None;
                Err(error)
            }
        }
    }

    fn set_row_regex_filter(
        &mut self,
        filter: SetRowRegexFilter,
    ) -> Result<(), parsers::InvalidRegexError> {
        if filter.get_filter() == "" {
            self.row_options.regex_filter = None;
            return Ok(());
        }

        match parsers::parse_regex(filter.get_filter()) {
            Ok(parsed_filter) => {
                self.row_options.regex_filter = Some(parsed_filter);
                Ok(())
            }
            Err(error) => {
                self.row_options.regex_filter = None;
                Err(error)
            }
        }
    }

    fn set_row_filter_combination(&mut self, combination: SetRowFilterCombination) {
        self.row_options.filters_combination = match combination.get_combination().inner {
            Some(CombinationInner::and(_)) => Some(transformers::Combination::And),
            Some(CombinationInner::or(_)) => Some(transformers::Combination::Or),
            None => None,
        }
    }

    fn set_row_separators(
        &mut self,
        separators: SetRowSeparators,
    ) -> Result<(), parsers::InvalidFieldSeparatorError> {
        match parsers::parse_field_separators(separators.get_separators()) {
            Ok(parsed_separators) => {
                self.row_options.separators = Some(parsed_separators);
                Ok(())
            }
            Err(error) => {
                self.row_options.separators = None;
                Err(error)
            }
        }
    }

    fn set_row_regex_separator(
        &mut self,
        separator: SetRowRegexSeparator,
    ) -> Result<(), parsers::InvalidRegexError> {
        if separator.get_separator() == "" {
            self.row_options.regex_separator = None;
            return Ok(());
        }

        match parsers::parse_regex(separator.get_separator()) {
            Ok(parsed_separator) => {
                self.row_options.regex_separator = Some(parsed_separator);
                Ok(())
            }
            Err(error) => {
                self.row_options.regex_separator = None;
                Err(error)
            }
        }
    }

    fn handle_message(&mut self, ctx: &mut ws::WebsocketContext<WebsocketConnection>, data: Bytes) {
        match FromClient::parse_from_bytes(&data.to_vec()) {
            Ok(message) => match message.inner {
                Some(FromClientInner::initialize(initial_values)) => {
                    match self.initialize(initial_values) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_column_index_filters(set_column_index_filters)) => {
                    match self.set_column_index_filters(set_column_index_filters) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_column_regex_filter(set_column_regex_filter)) => {
                    match self.set_column_regex_filter(set_column_regex_filter) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_column_filter_combination(
                    set_column_filter_combination,
                )) => {
                    self.set_column_filter_combination(set_column_filter_combination);

                    if let Err(error) = self.send_csvs(ctx) {
                        self.send_error(ctx, error);
                    }
                }
                Some(FromClientInner::set_column_separators(set_column_separators)) => {
                    match self.set_column_separators(set_column_separators) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_column_regex_separator(set_column_regex_separator)) => {
                    match self.set_column_regex_separator(set_column_regex_separator) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_row_index_filters(set_row_index_filters)) => {
                    match self.set_row_index_filters(set_row_index_filters) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_row_regex_filter(set_row_regex_filter)) => {
                    match self.set_row_regex_filter(set_row_regex_filter) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_row_filter_combination(set_row_filter_combination)) => {
                    self.set_row_filter_combination(set_row_filter_combination);

                    if let Err(error) = self.send_csvs(ctx) {
                        self.send_error(ctx, error);
                    }
                }
                Some(FromClientInner::set_row_separators(set_row_separators)) => {
                    match self.set_row_separators(set_row_separators) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                Some(FromClientInner::set_row_regex_separator(set_row_regex_separator)) => {
                    match self.set_row_regex_separator(set_row_regex_separator) {
                        Err(error) => self.send_error(ctx, error),
                        Ok(()) => {
                            if let Err(error) = self.send_csvs(ctx) {
                                self.send_error(ctx, error);
                            }
                        }
                    }
                }
                None => {
                    self.send_error(ctx, EmptyMessageError);
                }
            },
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

    fn send_full_continuation_frame(
        &mut self,
        ctx: &mut ws::WebsocketContext<WebsocketConnection>,
    ) {
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
                System::current().stop();
                return;
            }

            ctx.ping(b"");
        });
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebsocketConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Nop) => {}
            Ok(ws::Message::Continuation(Item::FirstText(data))) => {
                self.set_first_frame_part(data);
            }
            Ok(ws::Message::Continuation(Item::FirstBinary(data))) => {
                self.set_first_frame_part(data);
            }
            Ok(ws::Message::Continuation(Item::Continue(additional_data))) => {
                self.set_frame_part(additional_data);
            }
            Ok(ws::Message::Continuation(Item::Last(additional_data))) => {
                self.set_last_frame_part(additional_data);
                self.send_full_continuation_frame(ctx);
            }
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
            }
            Ok(ws::Message::Binary(data)) => {
                self.handle_message(ctx, data);
            }
            Ok(ws::Message::Close(reason)) => {
                ctx.close(reason);
                ctx.stop();
                System::current().stop();
            }
            Err(error) => {
                log::error!("{}", error);
                ctx.close(Some(CloseReason::from(CloseCode::Error)));
                ctx.stop();
            }
        }
    }
}
