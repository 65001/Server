/*
    A port of CS3214 Project framework written by Godmar Back. 
    Port written by Abhishek Sathiabalan. 
    (C) August 20, 2022.
*/

use clap::{Arg, App, AppSettings};

mod server;
mod http;
mod credentials;

// cargo build && server_unit_test_pserv.py -vs ./../target/debug/server
// server_bench.py -s ./../target/debug/server
fn main() {
    //We use clap to build an argument parser
    let matches = App::new("Personal Server")
    .setting(AppSettings::ArgRequiredElseHelp)
    .version("0.1.0")
    .author("Abhishek Sathiabalan")
    .about("My port of my CS3214 assignment in Rust")
    .arg(Arg::with_name("port")
        .short('p')
        .takes_value(true)
        .help("port number to bind to")
        .required(true))
    .arg(Arg::with_name("rootdir")
        .short('R')
        .takes_value(true)
        .help("root directory from which to serve file")
        .required(true)
        )
    .arg(Arg::with_name("seconds")
        .short('e')
        .takes_value(true)
        .help("expiration time for tokens in seconds"))
    .arg(Arg::with_name("silent")
        .short('s')
        .takes_value(false)
        .help("Silent Mode"))
    .arg(Arg::with_name("fallback")
        .short('a')
        .takes_value(false)
        .help("Enables HTML5 fallback"))
    .get_matches();

    let mut fallback : bool = false;
    let mut silence : bool = true;
    let mut expiration_time : u64 = 24 * 60 * 60;
    let mut port: u32 = 10000;
    let mut path : String = ".".to_string();
    let server_secret : String = "TOP+SECRET/MESSAGE".to_string();

    //These arguments require some processing
    if let Some(c) = matches.get_one::<String>("port") {
        match c.parse::<u32>() {
            Ok(n) => port = n,
            Err(_) => panic!("Port flag is not set to a number."),
        }
    }

    if let Some(c) = matches.get_one::<String>("seconds") {
        match c.parse::<u64>() {
            Ok(n) => expiration_time = n,
            Err(_) => panic!("Seconds flag is not set to a number."),
        }
    }

    if let Some(c) = matches.get_one::<String>("rootdir") {
        path = c.to_string();
    }

    if matches.contains_id("silent") {
        silence = true;
    }

    if matches.contains_id("fallback") {
        fallback = true;
    }

    let configuration: server::Config  = server::Config {
        html5_fallback: fallback,
        silent_mode: silence,
        token_expiration_time: expiration_time,
        server_root: path,
        port_number: port,
        server_secret: server_secret
    };

    let server = server::Server::new(configuration);
    server.start();
    
}
