extern crate futures;
extern crate serde_json;

pub mod request_reader;
pub mod response_writer;

use std::env;
use std::net::SocketAddr;

const PORT_ENV: &str = "PORT";

pub fn get_bind_address() -> SocketAddr {
    let port: u16 = env::var(PORT_ENV)
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(8080);

    ([0, 0, 0, 0], port).into()
}