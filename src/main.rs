use std::env;
use std::fmt;
use std::fs;
use std::os::unix::net::UnixListener;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::process::exit;
use std::process::Command;
use std::str;
use std::thread;
use std::time;

const DIRECTORY_NAME: &str = "devContainerAutoForward";
const MAIN_SOCKET_NAME: &str = "main.sock";
const CONTAINER_PATH: &str = "/devContainer";

struct ContainerState {
    container_id: String,
    socket: UnixListener,
}

fn parse_input() -> (String, String) {
    let args = env::args().collect::<Vec<String>>();
    if args.len() < 3 {
        eprintln!("ERROR: Please provide Operation and the Container Id");
        exit(1);
    }
    if args.len() > 3 {
        eprintln!("ERROR: Too many Arguments");
        exit(1);
    }
    let mode = args.get(1).expect("ERROR: Unable to read Container Id");
    let container_id = args.get(2).expect("ERROR: Unable to read Container Id");
    return (mode.clone(), container_id.clone());
}

fn create_command_socket(directory_name: PathBuf) -> UnixListener {
    let socket_path = directory_name.join(MAIN_SOCKET_NAME);
    if socket_path.exists() {
        fs::remove_file(socket_path.clone()).expect("Unable to delete old Socket!");
    }
    let stream =
        UnixListener::bind(socket_path.clone()).expect("ERROR: Unable to create UNIX Socket");
    stream
}

fn create_base_directory(container_id: &str) -> PathBuf {
    let mut path = env::temp_dir().join(DIRECTORY_NAME);
    if !path.exists() {
        fs::create_dir(path.clone()).expect("ERROR: Unable to create container Directory.");
    }
    let mut permissions = fs::metadata(path.clone())
        .expect(
            format!(
                "ERROR: Unable to read permissions of Base Directory: {}",
                path.clone().display()
            )
            .as_str(),
        )
        .permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path.clone(), permissions)
        .expect("ERROR: Unable to Set Permission to base Directory");
    path = path.join(container_id.clone());
    if !path.exists() {
        fs::create_dir(path.clone()).expect("ERROR: Unable to create container Directory.");
    }
    let mut permissions = fs::metadata(path.clone())
        .expect(
            format!(
                "ERROR: Unable to read permissions of Base Directory: {}",
                path.clone().display()
            )
            .as_str(),
        )
        .permissions();
    permissions.set_readonly(false);
    fs::set_permissions(path.clone(), permissions)
        .expect("ERROR: Unable to Set Permission to base Directory");
    path
}

fn host(container_id: String) {
    let base_directory = create_base_directory(&container_id);
    let socket = create_command_socket(base_directory.clone());
    let state = ContainerState {
        container_id,
        socket,
    };
    for stream in state.socket.incoming() {
        match stream {
            Ok(stream) => println!("INFO: {:?}", stream),
            Err(err) => eprintln!("ERROR: {:?}", err),
        }
    }
}
#[derive(Debug, PartialEq, Clone)]
enum Protocol {
    TCP,
    UDP,
}

impl fmt::Display for Protocol {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Protocol::TCP => write!(f, "{}", "TCP"),
            Protocol::UDP => write!(f, "{}", "UCP"),
        }
    }
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
        println!(
            "{proto:#?} {port} {app}",
            proto = item.protocol,
            port = item.port,
            app = item.app
        );
        port_list.push(item);
    }
    port_list
}

fn send_new_port(port: ListenPort) {
    println!(
        "INFO: New Open Port\nPort: {pro}{port}\nRunning: {app}",
        pro = port.protocol,
        port = port.port,
        app = port.app
    );
}
fn send_close_port(port: ListenPort) {
    println!("INFO: Closing Port {port}", port = port.port);
}

fn port_manager(stream: UnixStream) {
    let mut open_port_list: Vec<ListenPort> = Vec::new();

    loop {
        let new_list = detect_open_port();
        for port in new_list.clone() {
            if !open_port_list.contains(&port) {
                send_new_port(port.clone());
                open_port_list.push(port.clone());
            }
        }
        for (i, port) in open_port_list.clone().iter().enumerate() {
            if !new_list.contains(&port) {
                send_close_port(port.clone());
                open_port_list.remove(i);
            }
        }
        thread::sleep(time::Duration::from_secs(1));
    }
}

fn container(container_id: String) {
    let socket = env::temp_dir()
        .join(CONTAINER_PATH)
        .join(container_id.clone())
        .join(MAIN_SOCKET_NAME);
    let mut stream = UnixStream::connect(socket).expect("Unable to connect to main Socket");
    thread::spawn(|| port_manager(stream));
}

fn main() {
    let (mode, container_id) = parse_input();
    match mode.as_str() {
        "host" => host(container_id),
        "client" => container(container_id),
        _ => println!("ERROR: UNKNOWN MODE"),
    }
}
