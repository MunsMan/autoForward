pub const CLOSE: u8 = 0b0000;
pub const TCP: u8 = 0b0100;
pub const UDP: u8 = 0b0010;
pub const CREATE_TCP: u8 = 0b1100;
pub const CREATE_UDP: u8 = 0b1010;

pub enum NetworkProtocol {
    TCP,
    UDP,
}

pub struct UnixMessage {
    pub header: UnixHeader,
    pub message: Vec<u8>,
}

pub struct UnixHeader {
    pub size: u32,
    pub function: u8,
    pub port: u16,
}

pub fn read_header(header: [u8; 8]) -> Result<UnixHeader, String> {
    let size: u32 = ((header[0] as u32) << 24)
        | ((header[1] as u32) << 16)
        | ((header[2] as u32) << 8)
        | (header[3] as u32);
    let port: u16 = ((header[5] as u16) << 8) | (header[6] as u16);
    if port == 0 {
        return Err("The Port can't be 0".to_string());
    }
    let code: u8 = header[4];
    match code {
        CLOSE => {
            return Ok(UnixHeader {
                size,
                function: CLOSE,
                port,
            })
        }
        TCP => {
            return Ok(UnixHeader {
                size,
                function: TCP,
                port,
            })
        }
        UDP => {
            return Ok(UnixHeader {
                size,
                function: UDP,
                port,
            })
        }
        CREATE_TCP => {
            return Ok(UnixHeader {
                size,
                function: CREATE_TCP,
                port,
            })
        }
        CREATE_UDP => {
            return Ok(UnixHeader {
                size,
                function: CREATE_UDP,
                port,
            })
        }
        _ => return Err("Unsupported Function provided.".to_string()),
    }
}

pub fn encode_header(header: UnixHeader) -> [u8; 8] {
    let mut result: [u8; 8] = [0; 8];
    let (size, rest) = result.split_at_mut(4);
    size.copy_from_slice(&header.size.to_be_bytes());
    let (func, rest) = rest.split_at_mut(1);
    func.copy_from_slice(&header.function.to_be_bytes());
    let (port, _) = rest.split_at_mut(2);
    port.copy_from_slice(&header.port.to_be_bytes());
    result
}

pub fn encode_message(mut message: UnixMessage) -> Vec<u8> {
    let mut result = Vec::new();
    let header = encode_header(message.header);
    result.append(&mut header.to_vec());
    result.append(&mut message.message);
    return result;
}
