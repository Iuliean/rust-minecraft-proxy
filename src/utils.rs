use std::{usize};
use byteorder::ReadBytesExt;
use std::io::{Cursor, ErrorKind,Error};
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


pub fn read_var_int(buff: &mut impl ReadBytesExt)-> Result<i32, std::io::Error>{
    let mut value: i32 = 0;
    let mut pos: i32 = 0;    
    loop{
        let current_byte = match buff.read_u8(){
            Ok(value) => value,
            Err(e) => return Err(e)
        };

        value |= i32::from(current_byte & 0b0111_1111) << pos;

        if current_byte & 0b1000_0000 == 0 {
            break;
        }
        
        pos += 7;

        if pos >= 32 {return Err(Error::new(ErrorKind::InvalidData, "Number too big"));}
    }
    Ok(value)
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

pub fn read_string_255(buff: &mut impl ReadBytesExt) -> Result<String, std::io::Error>{
    let size = match read_var_int(buff){
        Ok(val) => val,
        Err(e) => return Err(e)
    };

    let mut str_buff:String = String::new();
    
    for i in 0..size as usize{
        match buff.read_u8(){
            Ok(v) => str_buff.push(v as char),
            Err(e)=> return Err(e)
        }
    }

    Ok(str_buff)
}

pub fn tokenize_to_packets(buff: &[u8]) -> Result<Vec<&[u8]>, std::io::Error>{
    let mut result: Vec<&[u8]> = Vec::new();
    let mut cr = Cursor::new(buff);
    while (cr.position() as usize) < buff.len(){
        match read_var_int(&mut cr){
            Ok(size) =>{
                let curr:usize = cr.position() as usize;
                let end:usize = curr + size as usize;
                if (curr + size as usize) > buff.len(){
                    result.push(&buff);
                    break;
                }
                if (size as usize) < buff.len() {
                    result.push(&buff[curr..end]);
                    cr.set_position(end as u64);
                }
            },
            Err(e) => return Err(e)
        };
        
    }
    let mut i: usize = 0;
    while i < result.len(){
        if result[i].is_empty(){
            result.remove(i);
            continue;
        }
        i = i+1 ;
    }
    Ok(result)
}   

