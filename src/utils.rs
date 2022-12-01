use std::io::{Error, ErrorKind};

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

        if (current_byte & 0b1000_0000) == 0 {break;}

        pos += 7;

        if pos > 32 {return None}
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

    Ok(result)
}   

pub trait Packet{
    fn parse(buff: &[u8]) -> Result<Self, &'static str> where Self: Sized;
    fn make_string(&self) -> String;
}

pub mod Client{
    use crate::utils;
    use num_derive::FromPrimitive;

    #[derive(FromPrimitive)]
    pub enum StatusPacketId{
        Handshake = 0x00,
    } 
    
    enum LoginPacketId{
        Start   = 0x00,
    }

    enum PlayPacketId{

    }
    
    #[derive(Debug)]
    pub struct HandshakePacket{
        protocol_version: i32,
        server_address: String,
        server_port: u16,
        next_state: i32
    }

    impl utils::Packet for HandshakePacket{
        fn parse(mut packet_buff: &[u8]) -> Result<Self, &'static str>{
            let packet = HandshakePacket{
                protocol_version:match utils::read_var_int(packet_buff){
                        Some(value) => {packet_buff = value.1; value.0},
                        None => return Err("Failed to parse protocol version.")
                    },
                server_address:match utils::read_string_255(packet_buff){
                        Some(value) => {packet_buff = value.1; value.0},
                        None => return Err("Couldn't parse string")
                    },
                server_port: {let (port, packet_buff) = utils::read_u16(packet_buff); port},
                next_state: match utils::read_var_int(packet_buff){
                    Some(value) => {packet_buff = value.1; value.0},
                    None => return Err("Failed to parse next_state")
                    }
            };
            Ok(packet)
        }

        fn make_string(&self) -> String{
            String::from(format!("protocol_version {}, to server: {}:{}",
                                    self.protocol_version,
                                    self.server_address,
                                    self.server_port))
        }
    }


}
