use std::net::{TcpListener, TcpStream};
use std::thread;

use crate::http::Transaction;

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
        let mut transaction : Transaction = Transaction::new();
        transaction.http_handle_transaction(stream);
    }

    pub fn stop(&self) {}
}