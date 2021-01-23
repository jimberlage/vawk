use actix_web::web;
use base64;
use serde_json;
use std::process::ExitStatus;

const MAX_CHUNK_SIZE: usize = 8 * 1_048_576;
const MAX_OUTPUT_SIZE: usize = 256 * 1_048_576;

#[derive(Debug)]
pub enum EncodingError {
    JSON(serde_json::Error),
    TooLarge,
}

impl From<serde_json::Error> for EncodingError {
    fn from(error: serde_json::Error) -> EncodingError {
        EncodingError::JSON(error)
    }
}

fn encode_stderr(stderr: &Vec<u8>) -> Result<String, EncodingError> {
    if stderr.len() > MAX_OUTPUT_SIZE {
        return Err(EncodingError::TooLarge);
    }

    Ok(base64::encode(stderr))
}

fn encode_stdout(stdout: &Vec<Vec<Vec<u8>>>) -> Result<String, EncodingError> {
    let mut output_size = 0usize;
    let mut base64_encoded = vec![];

    for line in stdout {
        let mut base64_encoded_line = vec![];

        for row in line {
            output_size += row.len();
            if output_size > MAX_OUTPUT_SIZE {
                return Err(EncodingError::TooLarge);
            }

            base64_encoded_line.push(base64::encode(row));
        }

        base64_encoded.push(base64_encoded_line);
    }

    Ok(serde_json::to_string(&base64_encoded)?)
}

fn chunked(encoded: &str, id: usize, event_type: &str) -> Vec<web::Bytes> {
    let mut chunk_size = 0usize;
    let mut chunks = vec![];
    let mut index = 0;
    let mut total = encoded.len() / MAX_CHUNK_SIZE;
    if encoded.len() % MAX_CHUNK_SIZE > 0 {
        total += 1;
    }
    let mut chunk: Vec<char> = format!("event: {}\ndata: {{\"index\": {}, \"total\": {}}}\ndata: ", event_type, index, total).chars().collect();

    // With base64 encoding & JSON, each char is one byte.
    // Each character is guaranteed to be ASCII.
    for c in encoded.chars() {
        chunk_size += 1;
        chunk.push(c);

        if chunk_size == MAX_CHUNK_SIZE {
            for _ in 0..2 {
                chunk.push('\n');
            }
            chunks.push(web::Bytes::from(chunk.iter().collect::<String>()));
            index += 1;
            chunk = format!("event: {}\ndata: {{\"index\": {}, \"total\": {}, \"id\": {}}}\ndata: ", event_type, index, total, id).chars().collect();
        }
    }

    if !chunk.is_empty() {
        for _ in 0..2 {
            chunk.push('\n');
        }
        chunks.push(web::Bytes::from(chunk.iter().collect::<String>()));
    }

    chunks
}

pub fn stdout_chunks(stdout: &Vec<Vec<Vec<u8>>>, id: usize) -> Result<Vec<web::Bytes>, EncodingError> {
    let encoded = encode_stdout(stdout)?;
    Ok(chunked(&encoded, id, "stdout"))
}

pub fn stderr_chunks(stderr: &Vec<u8>, id: usize) -> Result<Vec<web::Bytes>, EncodingError> {
    let encoded = encode_stderr(stderr)?;
    Ok(chunked(&encoded, id, "stderr"))
}

pub fn status_message(status: &ExitStatus, id: usize) -> web::Bytes {
    web::Bytes::from(format!("event: status\ndata: {{\"status\": {}, \"id\": {}}}\n\n", status, id))
}