use auto_forward::*;
use std::env;
use std::net::TcpStream;
use std::process::Command;
use std::str;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::thread;
use std::time::Duration;

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
    let mut results = stdout.split('\n').collect::<Vec<&str>>();
    let header = results
        .remove(0)
        .split(' ')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<&str>>();
    let mut table = results
        .into_iter()
        .map(|row| {
            row.split(' ')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect::<Vec<&str>>()
        })
        .filter(|row| row.len() == header.len() + 1)
        .collect::<Vec<Vec<&str>>>();
    table.retain(|r| match r.last() {
        Some(l) => *l == "(LISTEN)",
        None => false,
    });
    let mut port_list: Vec<ListenPort> = Vec::new();
    for row in table {
        let port_str: &str = match row.get(header.len() - 1) {
            Some(port) => port.split(':').last().unwrap_or(port),
            None => continue,
        };
        let port = match port_str.parse::<u16>() {
            Ok(port) => port,
            Err(_) => continue,
        };
        let proto = match Protocol::decode(
            row[header
                .iter()
                .position(|x| *x == "NODE")
                .unwrap_or(header.len() - 2)],
        ) {
            Ok(protocol) => protocol,
            Err(_) => continue,
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

fn request_new_port(port: &ListenPort) -> Message {
    let function = match port.protocol {
        Protocol::TCP => Function::CreateTcp,
        Protocol::UDP => Function::CreateUdp,
    };
    create_message(port.port, function, port.app.clone().into_bytes())
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
            if !new_list.contains(port) {
                send_close_port(port.clone());
                open_port_list.remove(i);
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}

fn get_inital_connection(port: u16) -> TcpStream {
    loop {
        match TcpStream::connect(format!("host.docker.internal:{port}")) {
            Ok(stream) => return stream,
            Err(err) => {
                eprintln!("Unable to connect to Host\nERROR: {err}")
            }
        };
        thread::sleep(Duration::from_secs(5));
    }
}

fn main() {
    let port = env::args()
        .nth(1)
        .unwrap_or("28258".to_string())
        .parse::<u16>()
        .unwrap_or(28258);
    let stream = get_inital_connection(port);
    let (sender, receiver) = channel();
    let read_stream = stream.try_clone().expect("Unable to clone stream");
    let write_stream = stream.try_clone().expect("Unable to clone stream");
    let port_sender = sender.clone();
    let port_thread = thread::spawn(|| port_manager(port_sender));
    thread::spawn(|| client_read_stream(read_stream, sender));
    thread::spawn(|| client_write_stream(write_stream, receiver));
    port_thread.join().unwrap();
}
