use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt;
use std::io::prelude::*;
use std::net::{TcpListener, TcpStream};
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::thread;

#[derive(Debug, PartialEq)]
pub enum Function {
    CreateTcp,
    CreateUdp,
    Tcp,
    Udp,
    NewListener,
}
pub struct Header {
    message_size: u32,
    function: Function,
    port: u16,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    fn from_bytes(byte: [u8; 1]) -> Protocol {
        match byte {
            [1] => Protocol::TCP,
            [2] => Protocol::UDP,
            _ => Protocol::TCP,
        }
    }

    fn to_bytes(&self) -> [u8; 1] {
        match self {
            Protocol::TCP => [1],
            Protocol::UDP => [2],
        }
    }
}

pub struct Message {
    pub header: Header,
    pub body: Vec<u8>,
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
    connection: HashMap<u16, Connection>,
    receiver: Receiver<Message>,
    sender: Sender<Message>,
    default: Sender<Message>,
    receiver_connection: Receiver<Connection>,
}

#[derive(Clone)]
struct Connection {
    port: u16,
    host_port: u16,
    protocol: Protocol,
    app: String,
    connection: Sender<Message>,
}

impl Multiplexer {
    pub fn new(stream: TcpStream) -> Multiplexer {
        if !stream
            .nodelay()
            .expect("Unable to read delay Mode of Socket")
        {
            stream.set_nodelay(true).expect("Unable to enable nodelay");
        }
        stream
            .set_nonblocking(true)
            .expect("Unable to enable non Blocking");
        let (sender, receiver) = channel();
        let (default_sender, default_receiver) = channel();
        let (connection_sender, connection_receiver) = channel();
        let multi = Multiplexer {
            stream: RefCell::new(stream),
            connection: HashMap::new(),
            receiver,
            sender: sender.clone(),
            default: default_sender,
            receiver_connection: connection_receiver,
        };
        thread::spawn(move || {
            handle_unknown_port(default_receiver, sender.clone(), connection_sender.clone())
        });
        multi
    }

    fn add_connection(&mut self, new_connection: Connection) {
        self.connection
            .insert(new_connection.port.clone(), new_connection);
    }

    fn send_message(&self, message: Message) -> Result<usize, std::io::Error> {
        let mut size;
        {
            let mut stream = self.stream.borrow_mut();
            size = stream.write(&encode_header(&message.header))?;
            size += stream.write(&message.body)?;
        }
        println!(
            "Sending {port} {function:?}",
            port = message.header.port,
            function = message.header.function
        );
        Ok(size)
    }

    fn read_header(&self) -> Result<Header, std::io::Error> {
        let mut header_buffer = [0 as u8; 8];
        let size: usize;
        {
            let mut stream = match self.stream.try_borrow_mut() {
                Ok(stream) => stream,
                Err(_) => return Err(std::io::ErrorKind::InvalidData.into()),
            };
            size = stream.read(&mut header_buffer)?;
        }
        if size != 8 {
            return Err(std::io::ErrorKind::InvalidData.into());
        }
        Ok(decode_header(&header_buffer))
    }

    fn read_message(&self) -> Result<Message, std::io::Error> {
        let header = self.read_header()?;
        let mut message = Vec::with_capacity(header.message_size as usize);
        {
            self.stream.borrow_mut().read(&mut message)?;
        }
        println!(
            "INFO: Message\n\tHeader: {} {:#?} {}\n\tMessage:\n{:#?}",
            header.message_size,
            header.function,
            header.port,
            str::from_utf8(&message)
        );
        Ok(Message {
            header,
            body: message,
        })
    }

    fn is_readable(&mut self) -> bool {
        let mut buffer = [0 as u8; 8];
        let res = self.stream.borrow_mut().peek(buffer.as_mut());
        if res.is_ok() {
            match res.ok() {
                Some(i) => return i == 8,
                None => return false,
            }
        }
        false
    }

    fn handle_socket_message(&self, message: Message) {
        let status = match self.connection.get(&message.header.port) {
            Some(connection) => connection.connection.send(message),
            None => self.default.send(message),
        };
        match status {
            Ok(()) => {}
            Err(err) => eprintln!(
                "ERROR: Something went wrong with the handle_socket_message!\n{}",
                err
            ),
        }
    }

    pub fn run(mut self) {
        println!("Multiplexer is Running");
        loop {
            if self.is_readable() {
                match self.read_message() {
                    Ok(message) => self.handle_socket_message(message),
                    Err(err) => eprintln!("ERROR: {err}"),
                }
            }
            for message in self.receiver.try_iter() {
                println!("Message from Host:\n{message}");
                self.send_message(message)
                    .expect("Something went wrong while forwarding a message");
            }
            thread::yield_now();
        }
    }
}

pub fn encode_header(header: &Header) -> [u8; 8] {
    let mut result = [0 as u8; 8];
    let function = match header.function {
        Function::CreateTcp => 0b0000_1100 as u8,
        Function::CreateUdp => 0b0000_1010 as u8,
        Function::Tcp => 0b0000_0100 as u8,
        Function::Udp => 0b0000_0010 as u8,
        Function::NewListener => 0b0001_0000 as u8,
    };
    result[0] = header.message_size.to_be_bytes()[0];
    result[1] = header.message_size.to_be_bytes()[1];
    result[2] = header.message_size.to_be_bytes()[2];
    result[3] = header.message_size.to_be_bytes()[3];
    result[4] = function;
    result[5] = header.port.to_be_bytes()[0];
    result[6] = header.port.to_be_bytes()[1];
    result
}

fn decode_header(buffer: &[u8; 8]) -> Header {
    Header {
        message_size: u32::from_be_bytes([buffer[0], buffer[1], buffer[2], buffer[3]]),
        function: decode_function(buffer[4]),
        port: u16::from_be_bytes([buffer[5], buffer[6]]),
    }
}

fn decode_function(function: u8) -> Function {
    match function {
        0b0000_1100 => Function::CreateTcp,
        0b0000_1010 => Function::CreateUdp,
        0b0000_0100 => Function::Tcp,
        0b0000_0010 => Function::Udp,
        _ => todo!(),
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
        body: message.clone(),
    }
}

fn get_socket(port: u16) -> (TcpListener, u16) {
    match TcpListener::bind(format!("localhost:{}", port)) {
        Ok(socket) => (socket, port),
        Err(_) => get_socket(port + 1),
    }
}

fn tcp_listener(
    socket: TcpListener,
    multi_sender: Sender<Message>,
    label_port: Cell<u16>,
    listen_port: Cell<u16>,
    _receiver: Receiver<Message>,
) {
    println!(
        "Starting listening on port: {} as {}",
        listen_port.get(),
        label_port.get()
    );
    for stream in socket.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New Connection!");
                let mut buffer = Vec::new();
                match stream.read_to_end(&mut buffer) {
                    Ok(_) => {}
                    Err(err) => {
                        eprintln!("ERROR: TCPListener, unable to read Message\n{}", err);
                        continue;
                    }
                };
                println!("{:#?}", str::from_utf8(&buffer));
                multi_sender
                    .send(create_message(label_port.get(), Function::Tcp, buffer))
                    .expect("Unable to forward message!");
            }
            Err(err) => {
                eprintln!("ERROR: TCPListener, unable to read Message\n{}", err);
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
        host_port: port,
        protocol: Protocol::TCP,
        app: match str::from_utf8(&message.body) {
            Ok(s) => s.to_string(),
            Err(_) => "Unkown".to_string(),
        },
        connection: sender,
    };
    let label_port = Cell::new(message.header.port.clone());
    let listen_port = Cell::new(port.clone());
    thread::spawn(|| tcp_listener(socket, multi_sender, label_port, listen_port, receiver));
    connection_sender.send(connection);
}

fn setup_udp_listener(multi_sender: Sender<Message>, message: Message) {
    println!("Setup UDP Listener");
}

fn handle_unknown_port(
    receiver: Receiver<Message>,
    multi_sender: Sender<Message>,
    connection_sender: Sender<Connection>,
) {
    loop {
        for message in receiver.try_iter() {
            match message.header.function {
                Function::CreateTcp => {
                    setup_tcp_listener(multi_sender.clone(), message, connection_sender.clone())
                }
                Function::CreateUdp => setup_udp_listener(multi_sender.clone(), message),
                _ => eprintln!("Something Went Wrong here: {message}"),
            }
        }
    }
}
