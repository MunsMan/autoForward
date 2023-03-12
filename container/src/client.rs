use crate::protocol::*;
use std::io::prelude::*;
use std::io::Read;
use std::net::TcpListener;
use std::net::{TcpStream, UdpSocket};
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;

fn read_stream(socket: &mut UnixStream) -> Result<(UnixMessage, &mut UnixStream), &mut UnixStream> {
    let mut header: [u8; 8] = [0; 8];
    let mut num_read = socket.take(8).read(&mut header).unwrap();
    if num_read != 8 {
        return Err(socket);
    }
    let header = read_header(header).unwrap();
    let mut message: Vec<u8> = Vec::with_capacity(header.size as usize);
    num_read = socket
        .take(header.size as u64)
        .read_to_end(&mut message)
        .unwrap_or(0);
    if num_read != (header.size as usize) {
        return Err(socket);
    }
    Ok((UnixMessage { header, message }, socket))
}

fn forward_tcp(message: UnixMessage) -> Option<UnixMessage> {
    let mut tcp = TcpStream::connect(format!("localhost:{}", message.header.port)).unwrap();
    let write_bytes = tcp.write(&message.message).unwrap_or(0);
    if write_bytes == 0 {
        tcp.shutdown(std::net::Shutdown::Both);
        return None;
    }
    let mut response = Vec::new();
    let read_bytes = tcp.read(&mut response).unwrap_or(0);
    if read_bytes == 0 {
        tcp.shutdown(std::net::Shutdown::Both);
        return None;
    }
    tcp.shutdown(std::net::Shutdown::Both);
    return Some(UnixMessage {
        header: UnixHeader {
            size: read_bytes as u32,
            function: TCP,
            port: message.header.port,
        },
        message: response,
    });
}

fn server_handle_tcp(socket: &mut UnixStream, message: UnixMessage) -> Result<(), String> {
    match forward_tcp(message) {
        None => Err("Unable to forward Bytes".to_string()),
        Some(message) => {
            let write_bytes = socket.write(&encode_message(message)).unwrap_or(0);
            if write_bytes == 0 {
                Err("Unable to Send Bytes".to_string())
            } else {
                Ok(())
            }
        }
    }
}

fn forward_udp(message: UnixMessage) -> Option<UnixMessage> {
    let udp = UdpSocket::bind(format!("localhost:{}", message.header.port)).unwrap();
    let write_bytes = udp.send(&message.message).unwrap_or(0);
    if write_bytes == 0 {
        return None;
    }
    let mut response = Vec::new();
    let read_bytes = udp.recv(&mut response).unwrap_or(0);
    if read_bytes == 0 {
        return None;
    }
    return Some(UnixMessage {
        header: UnixHeader {
            size: read_bytes as u32,
            function: UDP,
            port: message.header.port,
        },
        message: response,
    });
}

fn server_handle_udp(socket: &mut UnixStream, message: UnixMessage) -> Result<(), String> {
    match forward_udp(message) {
        None => Err("Unable to forward Bytes".to_string()),
        Some(message) => {
            let write_bytes = socket.write(&encode_message(message)).unwrap_or(0);
            if write_bytes == 0 {
                Err("Unable to Send Bytes".to_string())
            } else {
                Ok(())
            }
        }
    }
}

fn server_handle_message(socket: &mut UnixStream, message: UnixMessage) {
    match message.header.function {
        TCP => server_handle_tcp(socket, message).unwrap(),
        UDP => server_handle_udp(socket, message).unwrap(),
        _ => {
            eprintln!("WARNING: unknown code {:#02b}", message.header.function)
        }
    }
}

pub fn server(socket_name: String) {
    let socket = UnixListener::bind(socket_name).expect("Unable to create Socket");
    socket
        .set_nonblocking(true)
        .expect("Cloud not set to non Blocking");
    let (mut connection, _socket_address) = socket
        .accept()
        .expect("Failed at accepting a connection on the unix listener");
    loop {
        match read_stream(&mut connection) {
            Err(_socket) => {}
            Ok((message, socket)) => server_handle_message(socket, message),
        }
    }
}

pub fn client(socket_name: String) {
    let mut socket = UnixStream::connect(socket_name).expect("Unable to Connect to Socket");
    socket
        .set_nonblocking(true)
        .expect("Unable to set the Socket Non-Blocking");
    loop {
        match read_stream(&mut socket) {
            Err(_socket) => {}
            Ok((message, socket)) => server_handle_message(socket, message),
        }
    }
}

fn create_listener(port: u16, protocol: NetworkProtocol) {
    match protocol {
        NetworkProtocol::TCP => {
            let mut stream = TcpListener::bind(format!("localhost:{port}"))
                .expect(format!("ERROR: Unable to create TCP Socket on Port {port}").as_str());
            stream.set_nonblocking(true).expect(
                format!("ERROR: Unable to set TCP Socket on Port {port} to Non blocking.").as_str(),
            )
        }
        NetworkProtocol::UDP => {}
    }
}
