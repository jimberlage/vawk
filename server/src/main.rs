#![feature(proc_macro_hygiene, decl_macro)]

extern crate actix_web;
extern crate base64;
extern crate futures;
extern crate nom;
extern crate regex;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate ulid;

mod byte_trie;
mod command_executor;
mod parsers;
mod server;
mod transformers;

use std::io;

#[actix_web::main]
async fn main() -> io::Result<()> {
    server::serve().await
}
