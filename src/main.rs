use std::env;
use std::fs;
use std::os::unix::net::UnixListener;
use std::path::PathBuf;
use std::process::exit;

const DIRECTORY_NAME: &str = "devContainer-AutoForward";
const MAIN_SOCKET_NAME: &str = "main.sock";

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
    println!(
        "SETUP:\n\tcontainer: {}\n\tsocket: {:?}",
        state.container_id,
        state
            .socket
            .local_addr()
            .expect("Coudn't get local address")
            .as_pathname()
    );
}

fn container(container_id: String) {}

fn main() {
    let (mode, container_id) = parse_input();
    match mode.as_str() {
        "host" => host(container_id),
        "client" => container(container_id),
        _ => println!("ERROR: UNKNOWN MODE"),
    }
}
