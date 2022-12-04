
use crate::utils;
use std::cell::RefCell;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use utils::{State};
use crate::packets::{Packet, server};
use crate::packets::client;

trait HandshakeConnection {
    fn input(&self) -> &TcpStream;
    fn output(&self) -> &TcpStream;
    fn state(&self) -> State;
    fn set_state(&self, new_state: State);
    fn log(&self, message: String);
    
    fn on_status(&self, buff: Vec<&[u8]>);
    fn on_login(&self, buff: Vec<&[u8]>);
    fn on_play(&self, buff: Vec<&[u8]>);

    fn run(&self) {
        self.handshake();
    }

    fn execute(&self, buff: Vec<&[u8]>){
        let state = self.state();
        match state{
            State::Status => self.on_status(buff),
            State::Login => self.on_login(buff),
            State::Play => self.on_play(buff),
            State::Unknown =>self.log("Warning!!!! State is Unknonwn. Warning!!!!!".to_string())
        }
    }

    fn handshake(&self) {
        let mut buff = [0; 4096];
        let mut input = self.input();
        let mut output = self.output();
        loop {
            {
                let bytes_read = input.read(&mut buff).unwrap();
                if bytes_read != 0 {
                    match utils::tokenize_to_packets(&buff[..bytes_read]){
                        Ok(tokens) => self.execute(tokens),
                        Err(_e) => ()
                    }
                    output.write_all(&buff[..bytes_read]).unwrap();
                }
                
            }
        }
    }
}

struct M2P {
    input: TcpStream,
    output: TcpStream,
    login_packet_sent: RefCell<bool>,
    state: Arc<Mutex<State>> 
}

struct S2P {
    input: TcpStream,
    output: TcpStream,
    state: Arc<Mutex<State>>
}

impl HandshakeConnection for M2P {
    fn input(&self) -> &TcpStream {
        &self.input
    }

    fn output(&self) -> &TcpStream {
        &self.output
    }

    fn state(&self) -> State{
        let g = self.state.lock().unwrap();
        *g
    }
    
    fn set_state(&self, new_state: State){
        let mut g = self.state.lock().unwrap();
        *g = new_state;
    }
    
    fn log(&self, message: String){
        println!("[M2P]:{message}");
    }

    fn on_status(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = &packet[0];
            let packet = &packet[1..];
            match client::StatusPacketId::from_u8(id) {
                    client::StatusPacketId::Handshake =>{
                        match client::HandshakePacket::parse(packet){
                            Ok(parsed_value) => {
                                self.log(parsed_value.make_string());
                                self.set_state( State::Login);
                            },
                            Err(e) => self.log(format!("Failed to parse HandshakePacket reason: {}", e))
                        };
                }
                client::StatusPacketId::Unknown => self.log(format!("Unknown status packet id: {:#02x}", id))
            }
        }
    }

    fn on_login(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = &packet[0];
            let packet = &packet[1..];
            match client::LoginPacketId::from_u8(id){
                client::LoginPacketId::Start =>{
                    if !*self.login_packet_sent.borrow(){
                        match client::LoginStart::parse(packet){
                            Ok(parsed_value) => {
                                    *self.login_packet_sent.borrow_mut() = true;
                                    self.log(parsed_value.make_string());
                            },
                            Err(e) => {self.log(format!("{:?}", packet));self.log(format!("Failed to parse LoginStart packet reason: {}", e))}
                        }
                    }
                }
                client::LoginPacketId::Unknonwn => self.log(format!("Unknown login packet id:{:#02x}", id))
            }
        }
    }

    fn on_play(&self, buff: Vec<&[u8]>){
        for packet in buff{ 
            let id = &packet[1];
            let packet = &packet[2..];
            match client::PlayPacketId::from_u8(id){
                client::PlayPacketId::SetPlayerPosition =>{
                    match client::SetPlayerPositionPacket::parse(packet){
                        Ok(parsed_value) =>{
                            self.log(parsed_value.make_string());
                        },
                        Err(e) => self.log(format!("Failed to Parse SetPlayerPosition reason: {}", e)) 
                    }
                },
                client::PlayPacketId::SetPlayerRotation =>{
                    match client::SetPlayerRotationPacket::parse(packet){
                        Ok(parsed_value) =>{
                            self.log(parsed_value.make_string());
                        },
                        Err(e) => self.log(format!("Failed to parse SetPlayerRotationPacket reason: {}", e))
                    }
                },
                client::PlayPacketId::KeepAlive => {
                    match client::KeepAlivePacket::parse(packet){
                        Ok(parsed_value) => {
                            self.log(parsed_value.make_string())
                        }
                        Err(e) => self.log(format!("Failed to parse KeepAlive reason: {}" ,e))
                    }
                },
                client::PlayPacketId::SetPLayerPosAndRot => match client::SetPlayerPosAndRotPacket::parse(packet) {
                    Ok(parsed_value) => {
                        self.log(parsed_value.make_string());
                    }
                    Err(e) => self.log(format!("Failed to parse SetPlayerPosAndRot reason: {}", e))
                }
                client::PlayPacketId::Unknonwn => self.log(format!("Unknown play packet id: {:#02x}", id))
            }
        }
    }
}

impl HandshakeConnection for S2P {
    fn input(&self) -> &TcpStream {
        &self.input
    }

    fn output(&self) -> &TcpStream {
        &self.output
    }
    
    fn state(&self) -> State{
        let g = self.state.lock().unwrap();
        *g
    }

    fn set_state(&self, new_state: State){
        let mut g = self.state.lock().unwrap();
        *g = new_state;
    }
    
    fn log(&self, message: String){
        println!("[S2P]:{message}");
    }

    fn on_status(&self, _buff: Vec<&[u8]>){
        self.log("Server on status".to_string());
    }

    fn on_login(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = &packet[0];
            let packet = &packet[1..];
            match server::LoginPacketId::from_u8(*id){
                server::LoginPacketId::LoginSuccess => {
                    self.log("___________________Login Success___________________".to_string());
                    self.set_state(State::Play)
                },
                server::LoginPacketId::Unknonwn => ()//self.log(format!("Unkown packet id: {}", id))
            }
        }
    }

    fn on_play(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = &packet[0];
            let packet = &packet[1..];
            if *id == 0x1C{
                self.log("ChunkData".to_string());
            }
        }
    }
}

pub struct Proxy {
    m2p: Arc<Mutex<M2P>>,
    s2p: Arc<Mutex<S2P>>,
}

impl Proxy {
    pub fn new() -> Proxy {
        let mc_listener = TcpListener::bind("0.0.0.0:25567")
            .expect("Cannot bind to default address: 0.0.0.0:25565");
        let mc = mc_listener.accept().unwrap().0;
        let server =
            TcpStream::connect("localhost:25566").expect("Failed to connect to: localhost:25566");
        let mc_clone = mc.try_clone().unwrap();
        let server_clone = server.try_clone().unwrap();
        let state = Arc::from(Mutex::from(State::Status));
        let state_clone = state.clone();
        Proxy {
            m2p: Arc::from(Mutex::from(M2P {
                input: mc,
                output: server,
                state: state,
                login_packet_sent: RefCell::new(false)   
            })),
            s2p: Arc::from(Mutex::from(S2P {
                input: server_clone,
                output: mc_clone,
                state: state_clone
            }))
        }
    }

    pub fn run(&self) {
        let r1 = self.m2p.clone();
        let r2 = self.s2p.clone();

        let t1 = thread::spawn(move || {
            let g = r1.lock().unwrap();
            g.run();
        });
        let t2 = thread::spawn(move || {
            let g = r2.lock().unwrap();
            g.run();
        });

        t1.join().unwrap();
        t2.join().unwrap();
    }
}
