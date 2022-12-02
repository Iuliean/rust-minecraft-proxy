#[derive(Copy,Clone, Debug)]
pub enum State{
    Status = 1,
    Login = 2,
    Play = 3,
    Unknown= 100
}

impl State{
    pub fn from_u8(input: u8) -> State{
        match input{
            1 => Self::Status,
            2 => Self::Login,
            3 => Self::Play,
            _ => Self::Unknown
        }
    }
}


pub fn read_var_int(buff: &[u8])-> Option<(i32, &[u8])>{
    let mut value: i32 = 0;
    let mut pos: i32 = 0;
    let mut it: usize = 0;
    let mut current_byte: u8;
    
    loop{
        if it < buff.len()
        {
            current_byte = buff[it];
        }
        else
        {
            return None;
        }
        it += 1;
        value |= i32::from(current_byte & 0b0111_1111) << pos;

        if current_byte & 0b1000_0000 == 0 {
            break;
        }
        
        pos += 7;

        if pos >= 32 {return None}
    }
    Some((value, &buff[it..]))
}

pub fn read_var_int_long(buff: &[u8]) -> Option<(i64, &[u8])>{
    let mut value: i64 = 0;
    let mut pos: i32 = 0;
    let mut it: usize = 0;
    let mut current_byte: u8;

    loop{
        if it < buff.len()
        {
            current_byte = buff[it];
        }
        else
        {
            return None;
        }
        it += 1;

        value |= i64::from(current_byte & 0b0111_1111) << pos;

        if (current_byte & 0b1000_0000) == 0{break;}
        
        pos += 7;

        if pos >=64 {return None}
    }
    Some((value, &buff[it..]))
}

pub fn read_string_255(mut buff: &[u8]) -> Option<(String, &[u8])>{
    let (size, mut buff) = match read_var_int(buff){
        Some(val) => val,
        None => return None
    };

    let mut str_buff: Vec<u8> = vec![];
    
    for i in 0..size as usize{
        str_buff.push(buff[0]);
        buff = &buff[1..];
    }

    Some((String::from_utf8(str_buff).unwrap(), buff))
}

pub fn read_u16(buff: &[u8]) -> (u16, &[u8]){
    let h1: u16 = buff[0] as u16;
    let h2: u16 = buff[1] as u16;

    ((h1 << 8) | h2, &buff[2..])
}

pub fn read_i64(buff: &[u8]) -> (i64, &[u8]){
    let mut ans: i64 = 0;
    for i in 0..8{
        ans = (ans << 8) | buff[i] as i64;
    }

    (ans, &buff[8..])
}

pub fn read_byte_array(buff: &[u8], len: usize) -> (Vec<u8>, &[u8]){
    let mut result = Vec::<u8>::new();
    for i in 0..len{
        result.push(buff[i]);
    }
    (result, &buff[len..])
} 

pub fn tokenize_to_packets(mut buff: &[u8]) -> Result<Vec<&[u8]>, String>{
    let mut result: Vec<&[u8]> = Vec::new();

    while !buff.is_empty(){
        let size;
        (size, buff) = match read_var_int(buff){
            Some(val) => val,
            None => return Err(String::from("Failed to read packet size"))
        };
        
        if size as usize >= buff.len(){
            result.push(buff);
            buff = &[];
        }
        else{
            result.push(&buff[..size as usize]);
            buff = &buff[size as usize.. ];
        }
        
    }
    let mut i: usize = 0;
    while i < result.len(){
        if result[i].is_empty(){
            result.remove(i);
            continue;
        }
        i+=1;
    }
    Ok(result)
}   

pub trait Packet{
    fn parse(buff: &[u8]) -> Result<Self, String> where Self: Sized;
    fn make_string(&self) -> String;
}

pub mod Client{
    use crate::utils;

    use super::{State, Packet, read_string_255, read_i64, read_var_int, read_byte_array};

    #[derive(Clone, Copy, Debug)]
    pub enum StatusPacketId{
        Handshake = 0x00,
        Unknown
    } 
    
    impl StatusPacketId{
        pub fn from_u8(input: &u8) -> StatusPacketId{
            match input{
                0x00 => Self::Handshake,
                _ => Self::Unknown            }
        }

    }
    #[derive(Clone, Copy)]
    pub enum LoginPacketId{
        Start   = 0x00,
        Unknonwn
    }

    impl LoginPacketId{
        pub fn from_u8(input: &u8) -> LoginPacketId{
            match input{
                0x00 => Self::Start,
                _ => Self::Unknonwn
            }
        }
    }
    enum PlayPacketId{

    }
    
    #[derive(Debug)]
    pub struct HandshakePacket{
        protocol_version: i32,
        server_address: String,
        server_port: u16,
        pub next_state: State
    }

    impl Packet for HandshakePacket{
        fn parse(mut packet_buff: &[u8]) -> Result<Self, String>{
            let packet = HandshakePacket{
                protocol_version:match utils::read_var_int(packet_buff){
                        Some(value) => {packet_buff = value.1; value.0},
                        None => return Err(String::from("Could not parse protocol_version"))
                    },
                server_address:match utils::read_string_255(packet_buff){
                        Some(value) => {packet_buff = value.1; value.0},
                        None => return Err(String::from("Could not parse server_address"))
                    },
                server_port: {
                    let port:u16;
                    (port, packet_buff) = utils::read_u16(packet_buff);
                    port
                },
                next_state:{
                    let state = State::from_u8(packet_buff[0]);
                    match state{
                        State::Unknown => return Err(format!("Invalid state enum value: {}", packet_buff[0])),
                        _ => state
                    }
                }
            };
            Ok(packet)
        }

        fn make_string(&self) -> String{
            String::from(format!("protocol_version {}, to server: {}:{} with next state {}",
                                    self.protocol_version,
                                    self.server_address,
                                    self.server_port,
                                    self.next_state as u8))
        }
    }
    #[derive(Debug)]
    pub struct LoginStart{
        player_name: String,
        sig_data: bool, //if false the next 5 fields are not sent
        timestamp: i64, //true 8 bytes value
        pub_key_len: i32, //varint
        pub_key: Vec<u8>,
        sig_len: i32, //varint
        sig: Vec<u8>,
        has_uuid: bool,
        uuid: i64 //only if has_uuid
    }

    impl Packet for LoginStart{
        fn parse(mut buff: &[u8]) -> Result<Self, String> where Self: Sized {
            let name: String;
            (name, buff)= match read_string_255(buff){
                Some(value) => value,
                None => return Err(String::from("Failed to parse player_name"))
            };
            let sig_data:bool = buff[0] == 0x01;
            buff = &buff[1..];
            let mut timestamp:i64 = -1;
            let mut pub_key_len: i32 = -1;
            let mut pub_key: Vec<u8> = Vec::new();
            let mut sig_len: i32 = -1;
            let mut sig:Vec<u8> = Vec::new();
            if sig_data{
                (timestamp, buff) = read_i64(buff);
                (pub_key_len, buff) = match read_var_int(buff){
                    Some(values) => values,
                    None => return Err(String::from("Failed to parse pub_key_len"))
                };
                (pub_key, buff) = read_byte_array(buff, pub_key_len as usize); 
                (sig_len, buff) = match read_var_int(buff){
                    Some(values) => values,
                    None => return Err(String::from("Failed to parse sig_len"))
                };

                (sig, buff) = read_byte_array(buff, sig_len as usize);
                
            }
            let has_uuid:bool = buff[0] == 0x01;
            let mut uuid:i64 = -1;
            if has_uuid{
                uuid = read_i64(buff).0;
            }
            Ok(LoginStart{
                player_name:name,
                sig_data: sig_data,
                timestamp: timestamp,
                pub_key_len: pub_key_len,
                pub_key: pub_key,
                sig_len: sig_len,
                sig: sig,
                has_uuid: has_uuid,
                uuid: uuid
            })
        }

        fn make_string(&self) -> String {
            if self.sig_data{
                return format!("Has sig data player_name:{} with UUID:{}. Full print {:?}", self.player_name, self.uuid, self)

            }
            format!("player_name:{} with UUID:{}", self.player_name, self.uuid)
        }
    }
}
