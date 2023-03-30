use crate::multiplexer::*;
use std::env;
use std::net::TcpListener;
use std::net::TcpStream;
use std::process::exit;
use std::process::Command;
use std::str;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::thread;
use std::time;

mod multiplexer;

fn parse_input() -> String {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 2 {
        eprintln!("ERROR: Please provide the Operation Mode");
        exit(1);
    }
    if args.len() > 2 {
        eprintln!("ERROR: Too many Arguments");
        exit(1);
    }
    let mode = args.get(1).expect("ERROR: Unable to read Container Id");
    return mode.clone();
}

fn host(port: u16) {
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

fn request_new_port(port: &ListenPort) -> Message {
    let function = match port.protocol {
        Protocol::TCP => Function::CreateTcp,
        Protocol::UDP => Function::CreateUdp,
    };
    create_message(port.port, function, port.app.clone().into_bytes())
}

#[derive(PartialEq, Clone)]
struct ListenPort {
    port: u16,
    protocol: Protocol,
    app: String,
}

fn detect_open_port() -> Vec<ListenPort> {
    // lsof -i -P -n
    let output = Command::new("lsof")
        .arg("-i")
        .arg("-P")
        .arg("-n")
        .output()
        .expect("ERROR: unable to search for ports");
    let stdout = str::from_utf8(&output.stdout).expect("ERROR: Unable to parse stdout!");
    let mut results = stdout.split("\n").collect::<Vec<&str>>();
    let header = results
        .remove(0)
        .split(" ")
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();
    let mut table = results
        .into_iter()
        .map(|row| {
            row.split(" ")
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
        })
        .filter(|row| row.len() == header.len() + 1)
        .collect::<Vec<Vec<&str>>>();
    table = table
        .into_iter()
        .filter(|r| match r.last() {
            Some(l) => l.to_string() == "(LISTEN)",
            None => false,
        })
        .collect();
    let mut port_list: Vec<ListenPort> = Vec::new();
    for row in table {
        let port_str: &str = match row.get(header.len() - 1) {
            Some(port) => port.split(":").last().unwrap_or(port),
            None => continue,
        };
        let port = match port_str.parse::<u16>() {
            Ok(port) => port,
            Err(_) => continue,
        };
        let proto = match row[header
            .iter()
            .position(|x| x.to_string() == "NODE")
            .unwrap_or(header.len() - 2)]
        {
            "TCP" => Protocol::TCP,
            "UDP" => Protocol::UDP,
            _ => continue,
        };
        let app = match row.first() {
            Some(app) => app.to_string(),
            None => "Unkown".to_string(),
        };
        let item = ListenPort {
            port,
            protocol: proto,
            app,
        };
        port_list.push(item);
    }
    port_list
}

fn send_close_port(port: ListenPort) {
    println!("INFO: Closing Port {port}", port = port.port);
}

fn port_manager(sender: Sender<Message>) {
    let mut open_port_list: Vec<ListenPort> = Vec::new();

    loop {
        let new_list = detect_open_port();
        for port in new_list.clone() {
            if !open_port_list.contains(&port) {
                let port1 = port.clone();
                println!(
                    "INFO: New Open Port\nPort: {pro:?} {port}\nRunning: {app}",
                    pro = port1.protocol,
                    port = port1.port,
                    app = port1.app
                );
                sender.send(request_new_port(&port)).unwrap();
                open_port_list.push(port.clone());
            }
        }
        for (i, port) in open_port_list.clone().iter().enumerate() {
            if !new_list.contains(&port) {
                send_close_port(port.clone());
                open_port_list.remove(i);
            }
        }
        thread::sleep(time::Duration::from_secs(5));
    }
}

fn container(port: u16) {
    let mut threads = Vec::new();
    let stream = TcpStream::connect(format!("host.docker.internal:{port}"))
        .expect("ERROR: Unable to connect to Socket");
    println!("{}", stream.peer_addr().unwrap());
    let (sender, receiver) = channel();
    let read_stream = stream.try_clone().expect("Unable to clone stream");
    let write_stream = stream.try_clone().expect("Unable to clone stream");
    let port_sender = sender.clone();
    let port_thread = thread::spawn(|| port_manager(port_sender));
    let read_sender = sender.clone();
    threads.push(thread::spawn(|| {
        client_read_stream(read_stream, read_sender)
    }));
    threads.push(thread::spawn(|| {
        client_write_stream(write_stream, receiver)
    }));
    port_thread.join().unwrap();
}

fn main() {
    let mode = parse_input();
    let port = 3000;
    match mode.as_str() {
        "host" => host(port),
        "client" => container(port),
        _ => println!("ERROR: UNKNOWN MODE"),
    }
}
