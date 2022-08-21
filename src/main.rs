/*
    A port of CS3214 Project framework written by Godmar Back. 
    Port written by Abhishek Sathiabalan. 
    (C) August 20, 2022.
*/

use std::env;
use clap::{Arg, App, AppSettings};

struct Config {
    html5_fallback: bool, 
    silent_mode: bool,
    token_expiration_time: u32, 
    server_root : String, 
    server_secret: String
}


// cargo build
//./server.exe -p 8080 
fn main() {
    //We use clap to build an argument parser
    let matches = App::new("Personal Server")
    .setting(AppSettings::ArgRequiredElseHelp)
    .version("0.1.0")
    .author("Ab")
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
        .help("Silent Mode"))
    .arg(Arg::with_name("fallback")
        .short('a')
        .help("Enables HTML5 fallback"))
    .get_matches();

    let mut fallback : bool = false;
    let mut silence : bool = false;
    let mut expiration_time : u32 = 24 * 60 * 60;
    let mut path : String = ".".to_string();
    let server_secret : String = "TOP+SECRET/MESSAGE".to_string();

    let args: Vec<String> = env::args().collect();
    print!("{:?}", args);
}
