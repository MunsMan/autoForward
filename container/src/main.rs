mod client;
mod protocol;

use std::env;
use std::process::exit;

const SOCKET_NAME: &str = "DEV_AUTO_FORWARD";

fn main() {
    let args = env::args();
    if args.len() != 2 {
        eprintln!("ERROR: A Operation Type is required!");
        exit(1);
    }
    let key_word: String = args.into_iter().last().unwrap();
    if key_word == "client" {
        client::client(SOCKET_NAME.to_string());
    }
    if key_word == "server" {
        client::server(SOCKET_NAME.to_string());
    }
}
