use auto_forward::*;
use std::collections::HashMap;
use std::env;
use std::io::Read;
use std::io::Write;
use std::net::TcpStream;
use std::process::Command;
use std::str;
use std::sync::mpsc::channel;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::sync::RwLock;
use std::thread;
use std::time::Duration;

#[derive(PartialEq, Clone)]
struct ListenPort {
    port: u16,
    ip: String,
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
        let port_str = row.get(header.len() - 1).unwrap_or(&"a");
        let mut ip: String = match row.get(header.len() - 1) {
            Some(port) => port
                .split(':')
                .take(port_str.split(':').count() - 1)
                .collect::<Vec<&str>>()
                .join(":"),
            None => continue,
        };
        if ip == "*" {
            ip = "localhost".to_string();
        }
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
            ip,
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

fn port_manager(sender: Sender<Message>, port_register: Arc<RwLock<HashMap<u16, ListenPort>>>) {
    loop {
        let new_list = detect_open_port();
        for port in new_list.clone() {
            if !port_register.read().unwrap().contains_key(&port.port) {
                let port1 = port.clone();
                println!(
                    "INFO: New Open Port\nPort: {pro:?} {port}\nRunning: {app}",
                    pro = port1.protocol,
                    port = port1.port,
                    app = port1.app
                );
                sender.send(request_new_port(&port)).unwrap();
                port_register.write().unwrap().insert(port.port, port);
            }
        }
        for (port, listen_port) in port_register.read().unwrap().iter() {
            if !new_list.contains(listen_port) {
                send_close_port(listen_port.clone());
                port_register.write().unwrap().remove(port);
            }
        }
        thread::sleep(Duration::from_secs(5));
    }
}

fn handle_message(
    message: Message,
    sender: Sender<Message>,
    port_register: Arc<RwLock<HashMap<u16, ListenPort>>>,
) {
    if message.header.function == Function::Tcp {
        let request = message;
        thread::spawn(move || {
            let service = port_register
                .read()
                .unwrap()
                .get(&request.header.port)
                .unwrap()
                .clone();
            let mut stream = TcpStream::connect(format!("{}:{}", service.ip, request.header.port))
                .unwrap_or_else(|err| {
                    panic!(
                        "Error: Unable to connect to Socket localhost:{}\n{err}",
                        request.header.port
                    )
                });
            stream.write_all(&request.body).unwrap();
            let mut buffer = Vec::new();
            stream.read_to_end(&mut buffer).unwrap();
            sender
                .send(create_message(request.header.port, Function::Tcp, buffer))
                .unwrap();
        });
    } else {
        eprintln!(
            "INFO: This Function is currently not supported {:#?}",
            message.header.function
        );
    }
}

fn client_read_stream(
    stream: TcpStream,
    sender: Sender<Message>,
    port_register: Arc<RwLock<HashMap<u16, ListenPort>>>,
) {
    loop {
        match read_message(&stream) {
            Ok(message) => match message {
                Some(message) => handle_message(message, sender.clone(), port_register.clone()),
                None => {
                    eprintln!("Socket closed!");
                    break;
                }
            },
            Err(err) => eprintln!("Something went wrong with the message:\n{err}"),
        }
    }
}

// fn main() {
//     let new_list = detect_open_port();
//     for port in new_list.iter() {
//         println!("Port found: {} with ip: {}", port.port, port.ip);
//     }
// }

fn main() {
    let port = env::args()
        .nth(1)
        .unwrap_or("28258".to_string())
        .parse::<u16>()
        .unwrap_or(28258);
    let stream = TcpStream::connect(format!("host.docker.internal:{port}"))
        .expect("ERROR: Unable to connect to Socket");
    let port_register: Arc<RwLock<HashMap<u16, ListenPort>>> =
        Arc::new(RwLock::new(HashMap::new()));
    let (sender, receiver) = channel();
    let read_stream = stream.try_clone().expect("Unable to clone stream");
    let write_stream = stream.try_clone().expect("Unable to clone stream");
    let port_sender = sender.clone();
    let port_manager_register = port_register.clone();
    let port_thread = thread::spawn(move || port_manager(port_sender, port_manager_register));
    thread::spawn(move || client_read_stream(read_stream, sender, port_register));
    thread::spawn(|| client_write_stream(write_stream, receiver));
    port_thread.join().unwrap();
}
