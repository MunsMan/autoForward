use auto_forward::Multiplexer;
use std::env;
use std::net::TcpListener;
use std::process::exit;
use std::thread;

fn main() {
    let port = env::args()
        .nth(1)
        .unwrap_or("28258".to_string())
        .parse::<u16>()
        .unwrap_or(28258);
    let socket =
        TcpListener::bind(format!("127.0.0.1:{port}")).expect("ERROR: Unable to create Socket");
    println!("Listening on Port {port} for connections");
    let stream = match socket.accept() {
        Ok((stream, addr)) => {
            println!("Connection from {addr}");
            stream
        }
        Err(err) => {
            eprintln!("Unable to accept connection!\n{err}");
            exit(1);
        }
    };
    let multi = thread::spawn(|| Multiplexer::new(stream).run());
    multi.join().unwrap();
}
