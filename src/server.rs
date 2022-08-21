use std::net::{TcpListener, TcpStream};
use std::thread;

use std::sync::Arc;

use crate::http::Transaction;

#[derive(Debug)]
#[derive(Clone)]
pub struct Config {
    pub html5_fallback: bool, 
    pub silent_mode: bool,
    pub port_number: u32,
    pub token_expiration_time: u64, 
    pub server_root : String, 
    pub server_secret: String
}

#[derive(Debug)]
pub struct Server {
    configuration: Arc<Config>,
}


impl Server {
    pub fn new(value: Config) -> Self {
        Self {configuration: Arc::new(value) }
    }

    pub fn start(&self) {
        let listener = TcpListener::bind(format!("[::]:{}", self.configuration.port_number)).unwrap();
        for stream in listener.incoming() {
            let stream = stream.unwrap();
            let data : Arc<Config> = self.configuration.clone();
            thread::spawn(move || {
                Server::worker(stream,  data);
            });
        }

    }

    fn worker(mut stream : TcpStream, config : Arc<Config>) {
        //We don't handle the case when the client closes the connection all that well (?)
        let mut transaction : Transaction = Transaction::new(config);
        transaction.http_handle_transaction(stream);
    }

    pub fn stop(&self) {}
}