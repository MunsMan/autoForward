use std::env;
use std::fs;
use std::os::unix::net::UnixListener;
use std::process::exit;

struct ContainerState {
    container_id: String,
    socket: UnixListener,
}

fn parse_input() -> String {
    let args = env::args();
    if args.len() < 2 {
        eprintln!("ERROR: Please Provide the Container Id");
        exit(1);
    }
    if args.len() > 2 {
        eprintln!("ERROR: Too many Arguments");
        exit(1);
    }
    return args.last().expect("ERROR: Container Name not found");
}

fn create_command_socket(container_id: String) -> UnixListener {
    let path = env::temp_dir().join(container_id.clone());
    let file_path = path.join("main.sock");
    if !path.exists() {
        fs::create_dir(container_id).expect("ERROR: Unable to create container Directory.");
    }
    if file_path.exists() {
        fs::remove_file(path.clone()).expect("Unable to delete old Socket!");
    }
    let stream = UnixListener::bind(path).expect("ERROR: Unable to create UNIX Socket");
    stream
}

fn main() {
    let container_id = parse_input();
    let socket = create_command_socket(container_id.clone());
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
