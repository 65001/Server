use std::net::{TcpListener, TcpStream};
use std::io::{prelude::*, BufReader};
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct Config {
    pub html5_fallback: bool, 
    pub silent_mode: bool,
    pub port_number: u32,
    pub token_expiration_time: u32, 
    pub server_root : String, 
    pub server_secret: String
}

pub struct Server {
    configuration: Config,
}

impl Server {
    pub fn new(value: Config) -> Self {
        Self {configuration: value}
    }

    pub fn start(&self) {
        println!("{:#?}", self.configuration);
        let listener = TcpListener::bind(format!("[::]:{}", self.configuration.port_number)).unwrap();
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            thread::spawn(|| {
                Server::worker(stream);
            });
            println!("Connection established....");
        }
    }

    fn worker(mut stream : TcpStream) {
        //We don't handle the case when the client closes the connection all that well (?)
        let buf_reader = BufReader::new(&mut stream);
        let http_request: Vec<_> = buf_reader
            .lines()
            .map(|result| result.unwrap())
            .take_while(|line| !line.is_empty())
            .collect();

        println!("{:#?} Request: {:#?}",SystemTime::now(), http_request);
    }

    pub fn stop(&self) {}
}