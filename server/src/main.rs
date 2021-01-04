mod byte_trie;
mod command_executor;
mod encoding;
mod parsers;
mod server;
mod transformers;

use std::io;

#[actix_web::main]
async fn main() -> io::Result<()> {
    server::serve().await
}
