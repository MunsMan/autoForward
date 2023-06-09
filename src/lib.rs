use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use std::thread;

#[derive(Debug, PartialEq, Clone)]
pub enum Function {
    CreateTcp,
    CreateUdp,
    Tcp,
    Udp,
}

impl Function {
    fn encode(&self) -> u8 {
        match self {
            Function::CreateTcp => 0b0000_1100,
            Function::CreateUdp => 0b0000_1010,
            Function::Tcp => 0b0000_0100,
            Function::Udp => 0b0000_0010,
        }
    }

    fn decode(byte: u8) -> Function {
        match byte {
            0b0000_1100 => Function::CreateTcp,
            0b0000_1010 => Function::CreateUdp,
            0b0000_0100 => Function::Tcp,
            0b0000_0010 => Function::Udp,
            _ => {
                println!("{byte}");
                todo!()
            }
        }
    }
}

#[cfg(test)]
mod test_function {
    use super::*;

    #[test]
    fn ensure_inverse() {
        assert_eq!(
            Function::CreateTcp,
            Function::decode(Function::encode(&Function::CreateTcp))
        );
        assert_eq!(
            Function::CreateUdp,
            Function::decode(Function::encode(&Function::CreateUdp))
        );
        assert_eq!(
            Function::Tcp,
            Function::decode(Function::encode(&Function::Tcp))
        );
        assert_eq!(
            Function::Udp,
            Function::decode(Function::encode(&Function::Udp))
        );
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Header {
    pub message_size: u32,
    pub function: Function,
    pub port: u16,
}

impl Header {
    pub fn encode(&self) -> [u8; 8] {
        let mut result = [0; 8];
        result[0] = self.message_size.to_be_bytes()[0];
        result[1] = self.message_size.to_be_bytes()[1];
        result[2] = self.message_size.to_be_bytes()[2];
        result[3] = self.message_size.to_be_bytes()[3];
        result[4] = self.function.encode();
        result[5] = self.port.to_be_bytes()[0];
        result[6] = self.port.to_be_bytes()[1];
        result
    }

    pub fn decode(buffer: &[u8; 8]) -> Header {
        Header {
            message_size: u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]),
            function: Function::decode(buffer[4]),
            port: u16::from_be_bytes([buffer[5], buffer[6]]),
        }
    }
}

#[cfg(test)]
mod test_header {
    use super::*;

    #[test]
    fn decode_header_correctly() {
        let input_header: [u8; 8] = [0, 0, 0, 200, 0b1100, 0x0B, 0xB8, 0];
        let expected = Header {
            message_size: 200,
            function: Function::CreateTcp,
            port: 3000,
        };
        let result = Header::decode(&input_header);
        assert_eq!(expected.message_size, result.message_size);
        assert_eq!(expected.function, result.function);
        assert_eq!(expected.port, result.port);
    }
}

#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    pub fn decode(string: &str) -> Result<Protocol, String> {
        match string.to_uppercase().as_str() {
            "TCP" => Ok(Protocol::TCP),
            "UDP" => Ok(Protocol::UDP),
            _ => Err(format!("ERROR: Protocol: {string} is not defined")),
        }
    }
}

#[cfg(test)]
mod test_protocol {
    use super::*;

    #[test]
    fn decode_tcp() {
        assert_eq!(Protocol::TCP, Protocol::decode("TCP").unwrap());
        assert_eq!(Protocol::TCP, Protocol::decode("tcp").unwrap());
    }
    #[test]
    fn decode_udp() {
        assert_eq!(Protocol::UDP, Protocol::decode("UDP").unwrap());
        assert_eq!(Protocol::UDP, Protocol::decode("udp").unwrap());
    }
    #[test]
    #[should_panic]
    fn decode_unkown() {
        Protocol::decode("").unwrap();
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct Message {
    pub header: Header,
    pub body: Vec<u8>,
}

impl Message {
    fn encode(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.append(&mut self.header.encode().to_vec());
        buffer.append(&mut self.body.to_vec());
        buffer
    }
}

impl fmt::Display for Message {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Header:\n\tMessage Size: {}\n\tFunction: {:#?}\n\tPort: {}\nMessage:\n{}",
            self.header.message_size,
            self.header.function,
            self.header.port,
            str::from_utf8(&self.body)
                .unwrap_or(format!("Unable to decode {:#?}", self.body).as_str()),
        )
    }
}

pub struct Multiplexer {
    stream: RefCell<TcpStream>,
    connection: Arc<RwLock<HashMap<u16, Arc<Connection>>>>,
    receiver: Arc<Mutex<Receiver<Message>>>,
    _sender: Sender<Message>,
    default: Sender<Message>,
    receiver_connection: Arc<Mutex<Receiver<Connection>>>,
}

struct Connection {
    port: u16,
    _host_port: u16,
    _protocol: Protocol,
    _app: String,
    connection: Mutex<Sender<Message>>,
}

impl Multiplexer {
    pub fn new(stream: TcpStream) -> Multiplexer {
        if !stream
            .nodelay()
            .expect("Unable to read delay Mode of Socket")
        {
            stream.set_nodelay(true).expect("Unable to enable nodelay");
        }
        // stream
        //     .set_nonblocking(true)
        //     .expect("Unable to enable non Blocking");
        let (sender, receiver) = channel();
        let (default_sender, default_receiver) = channel();
        let (connection_sender, connection_receiver) = channel();
        let multi = Multiplexer {
            stream: RefCell::new(stream),
            connection: Arc::new(RwLock::new(HashMap::new())),
            receiver: Arc::new(Mutex::new(receiver)),
            _sender: sender.clone(),
            default: default_sender,
            receiver_connection: Arc::new(Mutex::new(connection_receiver)),
        };
        thread::spawn(move || {
            handle_unknown_port(default_receiver, sender.clone(), connection_sender)
        });
        multi
    }

    pub fn run(&self) {
        let read_stream = self.stream.borrow().try_clone().unwrap();
        let mut write_stream = self.stream.borrow().try_clone().unwrap();
        let connections = self.connection.clone();
        let default = self.default.clone();
        let read_thread = thread::spawn(move || loop {
            match read_message(&read_stream) {
                Ok(message) => match message {
                    Some(message) => handle_socket_message(connections.clone(), &default, message),
                    None => {
                        println!("Container closed Socket!");
                        break;
                    }
                },
                Err(err) => eprintln!("Something went wrong in the Stream\n{err}"),
            }
        });
        let receiver = self.receiver.clone();
        thread::spawn(move || {
            for message in receiver.lock().unwrap().iter() {
                send_message(&mut write_stream, message).unwrap();
            }
        });
        let receive_connection = self.receiver_connection.clone();
        let write_connections = self.connection.clone();
        thread::spawn(move || {
            for connection in receive_connection.lock().unwrap().iter() {
                write_connections
                    .write()
                    .unwrap()
                    .insert(connection.port, Arc::new(connection));
            }
        });
        read_thread.join().unwrap();
    }
}

fn read_header(stream: &TcpStream) -> Result<Option<Header>, std::io::Error> {
    let mut header_buffer = [0; 8];
    let size = stream.take(8).read(&mut header_buffer)?;
    if size == 0 {
        return Ok(None);
    }
    if size != 8 {
        return Err(std::io::ErrorKind::InvalidData.into());
    }
    Ok(Some(Header::decode(&header_buffer)))
}

pub fn read_message(stream: &TcpStream) -> Result<Option<Message>, std::io::Error> {
    let header = match read_header(stream)? {
        Some(header) => header,
        None => return Ok(None),
    };
    let mut body = Vec::new();
    let _size = stream
        .take(header.message_size.into())
        .read_to_end(&mut body)
        .unwrap();
    let message = Message { header, body };
    Ok(Some(message))
}

fn send_message(stream: &mut TcpStream, message: Message) -> Result<usize, std::io::Error> {
    let buffer = message.encode();
    let size = stream.write(&buffer)?;
    Ok(size)
}

fn handle_socket_message(
    connections: Arc<RwLock<HashMap<u16, Arc<Connection>>>>,
    default: &Sender<Message>,
    message: Message,
) {
    let status = match connections.read().unwrap().get(&message.header.port) {
        Some(connection) => connection.connection.lock().unwrap().send(message),
        None => default.send(message),
    };
    match status {
        Ok(()) => {}
        Err(err) => {
            eprintln!("ERROR: Something went wrong with the handle_socket_message!\n{err}")
        }
    }
}

#[cfg(test)]
mod test_handle_socket_message {
    use super::*;

    #[test]
    fn handling_default() {
        let connections: Arc<RwLock<HashMap<u16, Arc<Connection>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (sender, receiver) = channel::<Message>();
        let message = Message {
            header: Header {
                message_size: 0,
                function: Function::CreateTcp,
                port: 1234,
            },
            body: Vec::new(),
        };
        handle_socket_message(connections, &sender, message.clone());
        let res = receiver.recv().unwrap();
        assert_eq!(message, res);
    }

    #[test]
    fn handling_connection() {
        let connections: Arc<RwLock<HashMap<u16, Arc<Connection>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (sender, receiver) = channel::<Message>();
        let connection = Connection {
            port: 1234,
            _host_port: 1234,
            _protocol: Protocol::TCP,
            _app: "".to_string(),
            connection: Mutex::new(sender.clone()),
        };
        connections
            .write()
            .unwrap()
            .insert(1234, Arc::new(connection));
        let message = Message {
            header: Header {
                message_size: 0,
                function: Function::CreateTcp,
                port: 1234,
            },
            body: Vec::new(),
        };
        let (default, _) = channel();
        handle_socket_message(connections, &default, message.clone());
        let res = receiver.recv().unwrap();
        assert_eq!(message, res);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_header_correctly() {
        let input_header: [u8; 8] = [0, 0, 0, 200, 0b1100, 0x0B, 0xB8, 0];
        let expected = Header {
            message_size: 200,
            function: Function::CreateTcp,
            port: 3000,
        };
        let result = Header::decode(&input_header);
        assert_eq!(expected.message_size, result.message_size);
        assert_eq!(expected.function, result.function);
        assert_eq!(expected.port, result.port);
    }
}

fn create_header(port: u16, message_size: u32, function: Function) -> Header {
    Header {
        message_size,
        port,
        function,
    }
}

pub fn create_message(port: u16, function: Function, message: Vec<u8>) -> Message {
    let header = create_header(port, message.len() as u32, function);
    Message {
        header,
        body: message,
    }
}

fn get_socket(port: u16) -> (TcpListener, u16) {
    match TcpListener::bind(format!("localhost:{port}")) {
        Ok(socket) => (socket, port),
        Err(_) => get_socket(port + 1),
    }
}

fn tcp_listener(
    socket: TcpListener,
    multi_sender: Sender<Message>,
    label_port: Cell<u16>,
    _listen_port: Cell<u16>,
    receiver: Receiver<Message>,
) {
    let mut buffer = [0; 1024];
    for stream in socket.incoming() {
        match stream {
            Ok(mut stream) => {
                let mut message = Vec::new();
                loop {
                    let size = stream.read(&mut buffer).unwrap();
                    message.append(&mut buffer.to_vec());
                    if size < 1024 {
                        break;
                    }
                }
                multi_sender
                    .send(create_message(label_port.get(), Function::Tcp, message))
                    .expect("Unable to forward message!");
                let response = receiver.recv().unwrap();
                stream.write_all(&response.body).unwrap();
            }
            Err(err) => {
                eprintln!("ERROR: TCPListener, unable to read Message\n{err}");
                continue;
            }
        };
    }
}

fn setup_tcp_listener(
    multi_sender: Sender<Message>,
    message: Message,
    connection_sender: Sender<Connection>,
) {
    let (socket, port) = get_socket(message.header.port);
    let (sender, receiver) = channel();
    let connection = Connection {
        port: message.header.port,
        _host_port: port,
        _protocol: Protocol::TCP,
        _app: match str::from_utf8(&message.body) {
            Ok(s) => s.to_string(),
            Err(_) => "Unkown".to_string(),
        },
        connection: Mutex::new(sender),
    };
    let label_port = Cell::new(message.header.port);
    let listen_port = Cell::new(port);
    thread::spawn(|| tcp_listener(socket, multi_sender, label_port, listen_port, receiver));
    connection_sender.send(connection).unwrap();
}

fn setup_udp_listener(_multi_sender: Sender<Message>, _message: Message) {
    println!("Setup UDP Listener");
    todo!();
}

fn handle_unknown_port(
    receiver: Receiver<Message>,
    multi_sender: Sender<Message>,
    connection_sender: Sender<Connection>,
) {
    for message in receiver.iter() {
        match message.header.function {
            Function::CreateTcp => {
                setup_tcp_listener(multi_sender.clone(), message, connection_sender.clone());
            }
            Function::CreateUdp => setup_udp_listener(multi_sender.clone(), message),
            _ => eprintln!("ERROR: *handle_unknown_port* Wrong Header Function\n{message}\n\n"),
        }
    }
}

pub fn client_write_stream(mut stream: TcpStream, receiver: Receiver<Message>) {
    for message in receiver.iter() {
        match send_message(&mut stream, message) {
            Ok(_) => {}
            Err(err) => eprintln!("ERROR: Unable to forward Message:\n{err}"),
        };
    }
}
