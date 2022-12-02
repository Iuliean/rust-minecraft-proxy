
use crate::utils;
use crate::utils::Client::LoginStart;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;
use utils::{Packet, State,read_var_int};
use utils::Client::{HandshakePacket, StatusPacketId, LoginPacketId};



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
            State::Unknown =>self.log(String::from("Warning!!!! State is Unknonwn. Warning!!!!!"))
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
                    //self.log(format!("Received: {:?}", buff));
                    match utils::tokenize_to_packets(&buff[..bytes_read]){
                        Ok(tokens) => self.execute(tokens),
                        Err(e) => self.log(format!("{}", e))
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
            let id = StatusPacketId::from_u8(&packet[0]);
            let packet = &packet[1..];
            self.log(format!("Packet id: {:#02x}",id as u8));
            match id {
                    StatusPacketId::Handshake =>{
                        match HandshakePacket::parse(packet){
                            Ok(parsed_value) => {
                                self.log(parsed_value.make_string());
                                self.set_state(State::from_u8(parsed_value.next_state as u8));
                            },
                            Err(e) => self.log(format!("Failed to parse HandshakePacket reason: {}", e))
                        };
                }
                StatusPacketId::Unknown => self.log(format!("Unknown status packet id: {}", id as u8))
            }
        }
    }

    fn on_login(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = LoginPacketId::from_u8(&packet[0]);
            let packet = &packet[1..];
            match id{
                LoginPacketId::Start =>{
                    match LoginStart::parse(packet){
                        Ok(parsed_value) => {
                                self.log(parsed_value.make_string());
                                self.set_state(State::Play);
                        },
                        Err(e) => self.log(format!("Failed to parse LoginStart packet reason: {}", e))
                    }
                }
                LoginPacketId::Unknonwn => self.log(format!("Unknown login packet id:{}", id as u8))
            }
        }
    }

    fn on_play(&self, buff: Vec<&[u8]>){
        self.log(format!("Client sent {} packets: {:?}", buff.len(), buff));
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
        //println!("[S2P]:{message}");
    }

    fn on_status(&self, buff: Vec<&[u8]>){
        for packet in buff{
            let id = packet[0];
            let packet = &packet[1..];
            self.log(format!("Packet id: {:#02x}",id));
        }
    }

    fn on_login(&self, buff: Vec<&[u8]>){
        //println!("{:?}", buff);  
    }

    fn on_play(&self, _buff: Vec<&[u8]>){
        
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
                state: state
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
