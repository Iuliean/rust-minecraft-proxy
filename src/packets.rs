pub trait Packet{
    fn parse(buff: &[u8]) -> Result<Self, std::io::Error> where Self: Sized;
    fn make_string(&self) -> String;
}

pub mod Client{
    use std::{io::{Cursor, ErrorKind,Error}, collections::btree_map::Values};
    use byteorder::{BigEndian, ReadBytesExt};
    use super::Packet;
    use crate::utils::{State, read_string_255, read_var_int};


    #[derive(Clone, Copy, Debug)]
    pub enum StatusPacketId{
        Handshake = 0x00,
        Unknown
    } 
    
    impl StatusPacketId{
        pub fn from_u8(input: &u8) -> StatusPacketId{
            match input{
                0x00 => Self::Handshake,
                _ => Self::Unknown           
            }
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
    pub enum PlayPacketId{
        SetPlayerPosition = 0x14,
        SetPlayerRotation = 0x16,
        Unknonwn
    }

    impl PlayPacketId{
        pub fn from_u8(input: &u8) -> PlayPacketId{
            match input {
                0x14 => Self::SetPlayerPosition,
                0x16 => Self::SetPlayerRotation,
                _ => Self::Unknonwn
            }
        }
    }
    
    #[derive(Debug)]
    pub struct HandshakePacket{
        protocol_version: i32,
        server_address: String,
        server_port: u16,
        pub next_state: State
    }

    impl Packet for HandshakePacket{
        fn parse(packet_buff: &[u8]) -> Result<Self, std::io::Error>{
            let mut cr = Cursor::new(packet_buff);
            let packet = HandshakePacket{
                protocol_version:match read_var_int(&mut cr){
                        Ok(value) => value,
                        Err(e) => return Err(Error::new(e.kind(), 
                                                    format!("Failed to read protocol version reason: {}", e.to_string())))
                    },
                server_address:match read_string_255(&mut cr){
                        Ok(value) => value,
                        Err(e) => return Err(Error::new(e.kind(),
                                                        format!("Failed to read server address reason: {}", e.to_string())))
                    },
                server_port: match cr.read_u16::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                            format!("Failed to read server port reason: {}", e.to_string())))
                },
                next_state: match cr.read_u8(){
                    Ok(value) => match State::from_u8(value){
                        State::Unknown => return Err(Error::new(ErrorKind::InvalidData,
                                                                format!("Read invalid status vale of: {:#02x}", value))),
                        _ => State::from_u8(value)
                    },
                    Err(e) => return Err(Error::new(e.kind(),
                                                            format!("failed to read state reason: {}", e.to_string())))      
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
        timestamp: Option<i64>, //true 8 bytes value
        pub_key_len: Option<i32>, //varint
        pub_key: Option<Vec<u8>>,
        sig_len: Option<i32>, //varint
        sig: Option<Vec<u8>>,
        has_uuid: bool,
        uuid: Option<uuid::Uuid> //only if has_uuid
    }

    impl Packet for LoginStart{
        fn parse(buff: &[u8]) -> Result<Self, std::io::Error> {
            let mut cr = Cursor::new(buff);
            let name = match read_string_255(&mut cr){
                Ok(value) => value,
                Err(e) => return Err(Error::new(e.kind(),
                                                        format!("Failed to parse name reason: {}", e.to_string())))
            };
            let sig_data = match cr.read_u8(){
                Ok(value) => value == 0x01,
                Err(e) => return Err(Error::new(e.kind(),
                                                            format!("Failed to parse sig_data reason: {}", e.to_string())))
            };

            let (timestamp, pub_key_len, pub_key, sig_len, sig) = if sig_data{
                let timestamp = match cr.read_i64::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                                format!("Failed to parse timestamp reason: {}", e.to_string()))) 
                };
                let pub_key_len = match read_var_int (&mut cr){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                            format!("Failed to parse pub_key_len reason: {}", e.to_string())))
                };

                let pub_key = if pub_key_len > 0{
                    let vec = Vec::<u8>::from(&buff[cr.position() as usize..(cr.position()+ pub_key_len as u64) as usize]);
                    cr.set_position(cr.position() + pub_key_len as u64);
                    Some(vec)
                }else{
                    None
                };
                let sig_len = match read_var_int(&mut cr){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                            format!("Failed to parse sig_len reason: {}", e.to_string())))  
                };
                let sig = if sig_len > 0{
                    let vec = Vec::<u8>::from(&buff[cr.position() as usize..(cr.position()+ pub_key_len as u64) as usize]);
                    cr.set_position(cr.position() + sig_len as u64);
                    Some(vec)
                }else{
                    None
                };
                (Some(timestamp), Some(pub_key_len), pub_key, Some(sig_len), sig)
            } else{
                (None, None, None, None, None)
            };

            let has_uuid= match cr.read_u8(){
                Ok(value) => value == 0x01,
                Err(e) => return Err(Error::new(e.kind(),
                                                        format!("Failed to parse has_uuid reason: {}", e.to_string())))
            };

            let uuid = if has_uuid{
                match cr.read_u128::<BigEndian>(){
                    Ok(value) => Some(uuid::Uuid::from_u128(value)),
                    Err(e)=> return Err(Error::new(e.kind(), 
                                                            format!("Failed to parse uuid value reason: {}", e.to_string())))
                }
            }else{
                None
            };

            Ok(LoginStart {
                player_name: name, 
                sig_data: sig_data, 
                timestamp: timestamp, 
                pub_key_len: pub_key_len, 
                pub_key: pub_key, 
                sig_len: sig_len, 
                sig: sig, 
                has_uuid:has_uuid, 
                uuid:uuid })
        }

        fn make_string(&self) -> String {
            format!("{:?}", self)
        }
    }

    #[derive(Debug)]
    pub struct SetPlayerPositionPacket{
        pos_x: f64,
        pos_y: f64,
        pos_z: f64,
        on_ground: bool
    }

    impl Packet for SetPlayerPositionPacket {
        fn parse(buff: &[u8]) -> Result<Self, std::io::Error> where Self: Sized {
            let mut cr = Cursor::new(buff);
            let packet = SetPlayerPositionPacket{
                pos_x: match cr.read_f64::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse pos_x reason: {}", e.to_string())))
                },
                pos_y: match cr.read_f64::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse pos_y reason: {}", e.to_string())))
                },
                pos_z: match cr.read_f64::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse pos_z reason: {}", e.to_string())))
                },
                on_ground: match cr.read_u8(){
                    Ok(value) => value == 0x01,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse on_ground reason: {}", e.to_string())))
                }
            };
            Ok(packet)
        }

        fn make_string(&self) -> String{
            format!("x:{:.2} y:{:.2} z:{:.2} on_ground:{}", self.pos_x, self.pos_y, self.pos_z, self.on_ground)
        }
    }

    pub struct SetPlayerRotationPacket{
        yaw: f32,
        pitch: f32,
        on_ground: bool
    }

    impl Packet for SetPlayerRotationPacket{
        fn parse(buff: &[u8]) -> Result<Self, std::io::Error> where Self: Sized {
            let mut cr = Cursor::new(buff);
            let packet = SetPlayerRotationPacket{
                yaw: match cr.read_f32::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse yaw reason: {}", e.to_string())))
                },
                pitch: match cr.read_f32::<BigEndian>(){
                    Ok(value) => value,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse pitch reason: {}", e.to_string())))
                },
                on_ground: match cr.read_u8(){
                    Ok(value) => value == 0x01,
                    Err(e) => return Err(Error::new(e.kind(),
                                                    format!("Failed to parse on_ground reason: {}", e.to_string())))
                }
            };
            Ok(packet)
        }

        fn make_string(&self) -> String {
            format!("yaw:{:.2} pitch:{:.2} on_ground:{}", self.yaw, self.pitch, self.on_ground)
        }
    }
    
}
