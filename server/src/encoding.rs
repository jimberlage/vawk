use base64;
use serde_json;

const CHUNK_SIZE: usize = 8 * 1_048_576;
const MAX_OUTPUT_SIZE: usize = 256 * 1_048_576;

pub enum EncodingError {
    JSON(serde_json::Error),
    TooLarge,
}

impl From<serde_json::Error> for EncodingError {
    fn from(error: serde_json::Error) -> EncodingError {
        EncodingError::JSON(error)
    }
}

pub fn encode_stderr(stderr: &Vec<u8>) -> Result<String, EncodingError> {
    if stderr.len() > MAX_OUTPUT_SIZE {
        return Err(EncodingError::TooLarge);
    }

    Ok(base64::encode(stderr))
}

pub fn encode_stdout(stdout: &Vec<Vec<Vec<u8>>>) -> Result<String, EncodingError> {
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

pub struct ChildOutputIterator {
    output: dyn Iterator<Item = u8>
}

impl Iterator for ChildOutputIterator {
    type Item = String;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}