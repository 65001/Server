use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::io::Read;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::net::TcpStream;
use std::collections::HashMap;
use std::sync::Arc;

use crate::http::Method::*;
use crate::http::Version::*;
use crate::http::ResponseStatus::*;

use crate::credentials::Credentials;
use serde_json::Result;
use jwt_simple::prelude::*;
use crate::server::Config;

use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
#[derive(PartialEq)]
enum Method {
    HttpPost,
    HttpGet,
    HttpUnknown
}

#[derive(Debug)]
#[derive(PartialEq)]
enum Version {
    Http1_0,
    Http1_1
}

#[derive(Debug)]
#[derive(PartialEq)]
#[allow(dead_code)]
enum ResponseStatus {
    HttpOk = 200,
    HttpPartialContent = 206,
    HttpBadRequest = 400,
    HttpPermissionDenied = 403,
    HttpNotFound = 404,
    HttpMethodNotAllowed = 405,
    HttpRequestTimeout = 408,
    HttpRequestTooLong = 414,
    HttpInternalError = 500,
    HttpNotImplemented = 501,
    HttpServiceUnavailable = 503
}

#[derive(Debug)]
pub struct Transaction {
    method : Method,
    version: Version,

    path: Option<String>,
    body: Option<String>, 
    content_length: usize, 
    resp_status : ResponseStatus, 

    req_headers: HashMap<String, String>,

    cookies: HashMap<String, String>,
    resp_headers : Vec<String>, 
    resp_body : Vec<String>, 

    jwt: Option<String>, 

    range_start :u32, 
    range_end: u32, 

    config: Arc<Config>
}

impl Transaction {
    pub fn new(con : Arc<Config>) -> Self {
        Self {
            method : HttpUnknown,
            version : Http1_0,
            resp_status : HttpInternalError, 
            jwt: None,
            body: None,
            path: None,
            range_start: 0,
            range_end: 0,
            content_length: 0,
            resp_body: Vec::new(),
            resp_headers: Vec::new(),
            req_headers: HashMap::new(),
            cookies: HashMap::new(),
            config: con
        }
    }

    /*
    Parsing Section
    */
    fn parse_request<R: BufRead>(&mut self, r: &mut R) -> bool {
        let mut line = String::new();
        let mut len = r.read_line(&mut line).unwrap();
        
        if len < 2 {
            return false;
        }
        line.pop();
        line.pop();
        
        let split : Vec<&str> = line.split(" ").collect();

        //This means we don't have the Method, the path, or the Version
        //Therefore we MUST close the connection
        if split.len() < 3 {
            println!("SPLIT 123: {:?}, {:?}", split, line);
            return false;
        }

        if split[0] == "GET" {
            self.method = HttpGet;
        }
        else if split[0] == "POST" {
            self.method = HttpPost;
        }
        else {
            println!("SPLIT 456: {:?}", split);
        }

        self.path = Some(split[1].to_string());
        
        if split[2] == "HTTP/1.1" {
            self.version = Http1_1;
        }
        return true;
    }

    fn parse_headers<R: BufRead>(&mut self, r: &mut R) -> bool {
        for l in r.lines() {
            let mut line = l.unwrap();
            if line == "" {
                return true;
            }
            //We must be a header
            let split : Vec<&str> = line.split(":").collect();
            if split.len() < 2 {
                return false;
            }
            let key = split[0];
            let value = split[1].trim().to_string();

            match key {
                "Content-Length" => self.content_length = value.parse().unwrap(),
                "Cookie" => {
                    let data : String = value.parse().unwrap();
                    for entry in data.split(";") {
                        let fields : Vec<&str> = entry.split("=").collect();
                        let key = fields[0].to_string();
                        let value = fields[1].to_string();
                        if key == "auth" {
                            self.jwt = Some(value.clone());
                        }
                        self.cookies.insert(key, value);
                    }
                },
                "Range" => {},
                _ => {}
            }

            self.req_headers.insert( key.to_string(), value ); 
        }
        return true;
    }

    /*
    Sending Response Section
    */
    fn send_error<W: Write>(&mut self, status : ResponseStatus, message : &str, stream : W) -> bool{
        self.resp_body.push(message.to_string());
        self.add_header("Content-Type", "text/plain");
        self.resp_status = status;
        return self.send_response(stream);
    }

    fn start_response(&self) -> String {
        let mut string  : Vec<&str> = Vec::new();
        string.push("HTTP/1.1 ");
        match self.resp_status {
            HttpOk => string.push("200 OK"),
            HttpPartialContent => string.push("206 Partial Content"),
            HttpBadRequest => string.push("400 Bad Request"),
            HttpPermissionDenied => string.push("403 Permission Denied"),
            HttpNotFound => string.push("404 Not Found"),
            HttpMethodNotAllowed => string.push("405 Method Not Allowed"),
            HttpRequestTimeout => string.push("408 Request Timeout"),
            HttpRequestTooLong => string.push("414 Request Too Long"),
            HttpInternalError => string.push("500 Internal Server Error"),
            HttpNotImplemented => string.push("501 Not Implemented"),
            HttpServiceUnavailable => string.push("503 Service Unavailable")
        };
        string.push("\r\n");
        return string.join("");
    }


    fn send_response<W: Write>(&mut self, mut stream : W) -> bool {
        let mut sb : Vec<String> = Vec::new();
        sb.push(self.start_response());

        let body = self.resp_body.join("");
        
        self.add_header("Content-Length", &body.len().to_string());
        sb.push(self.resp_headers.join(""));

        sb.push("\r\n".to_string());
        sb.push(body);
        let response : String = sb.join("");

        if !self.config.silent_mode {
            println!("> {:#?}", self);
        }

        match stream.write(response.as_bytes()) {
            Ok(_) => {},
            Err(e) => println!("Failed sending response: {}", e),
        }
        stream.flush().unwrap();
        return true;
    }

    fn handle_api<W: Write>(&mut self, stream : W) {
        let path = self.path.as_ref().unwrap();
        if path == "/api/login" {
            self.handle_api_login(stream);
        }
        else if path == "/api/video" {

        }
    }

    fn verify_jwt(&self) -> bool {
        if self.jwt == None {
            return false;
        }
        //TODO: Add more JWT verification later
        return true;
    }

    fn generate_jwt<W: Write>(&mut self,stream : W, credentials : Credentials) -> bool{
        let key = HS256Key::from_bytes( self.config.server_secret.as_bytes());
        let duration = Duration::from_secs(self.config.token_expiration_time);
        let claims = Claims::create( duration )
            .with_subject(credentials.username);
        let token  = key.authenticate(claims);
        match token {
            Ok(v) =>  {
                let cookie_value : String = format!("auth={};Path=/", v);
                let cookie_value_sent : &str = &cookie_value[..];
                self.add_header("Set-Cookie", cookie_value_sent);
                self.resp_status = HttpOk;
                return self.send_response(stream);
            },
            Err(e) => panic!("Could not generate JWT: {}", e)
        }
    }

    fn handle_api_login<W: Write>(&mut self, stream : W) -> bool {
        if self.method == HttpGet {
            self.add_header("Content-Type", "application/json");

            if self.verify_jwt() {

            }
            else {
                self.resp_body.push("{}".to_string());
            }
            self.resp_status = HttpOk;
            return self.send_response(stream);
        }
        else if self.method == HttpPost {
            self.add_header("Content-Type", "application/json");
            if self.body == None {
                self.send_error(HttpBadRequest, "No Body sent on POST request to /api/login", stream);
                return false;
            }
            let json : String = self.body.as_ref().unwrap().to_string();
            let c: Result<Credentials> = serde_json::from_str(&json);
            match c {
                Err(_) => {
                    self.send_error(HttpBadRequest, "Missing key/value pairs in the request body.", stream);
                    return false;
                },
                Ok(d) => {
                    if !d.valid() {
                        //Invalid credentials
                        self.send_error(HttpPermissionDenied, "Access denied.", stream);
                        return false;
                    }
                    return self.generate_jwt(stream, d);
                }
            }
        }
        return self.send_error(HttpNotImplemented,"API Method not Implemented", stream);
    }

    fn handle_static_asset<W:Write>(&mut self, stream : W) -> bool {
        let req_path : &str = self.path.as_ref().unwrap();

        //IDOR Redirection Attack Prevention
        if req_path.contains("..") {
            return self.send_error(HttpNotFound, "This file could not be found.", stream);
        }

        let file_name = format!("{}{}",self.config.server_root, req_path);
        let index_name = format!("{}{}",self.config.server_root, "/index.html");

        if Path::new(&file_name.clone()).exists() {
            return self.send_file(file_name);
        }
        else if self.config.html5_fallback && Path::new(&index_name.clone()).exists() {
            return self.send_file(index_name);
        }
        else {
            let message = format!("File {} not found", file_name);
            return self.send_error(HttpNotFound, &message, stream);
        }
    }

    fn send_file(&mut self, fname : String) -> bool {
        let file_size = fs::metadata( fname.clone() );
        let mut size : u64 = 0;
        match file_size {
            Err(e) => {println!("Error: {}", e);},
            Ok(t) => {
                size = t.len();
            }
        }
        
        println!("File Size: {} for {}", size, fname);

        let start : u64 = 0; 
        let buffer_size : usize = (size - start).try_into().unwrap();;
        let mut buf = vec![0; buffer_size];
        
        let file =  File::open(fname);
        let contents : Option<String> = None;

        match file {
            Ok(mut f) => {
                let r  = f.read_exact(&mut buf);
                match r {
                    Err(e) => println!("Could not access file: {}", e),
                    Ok(c) => println!("Contents: {:?}", std::str::from_utf8(&buf))
                }
            },
            Err(e) => {
                println!("Could not access file: {}", e);
            }
        }

        return true;
    }

    fn add_header(&mut self,  key : &str,  value : &str) {
        self.resp_headers.push( key.to_string() + ": " + &value.to_string() + "\r\n" );
    }




    pub fn http_handle_transaction(&mut self, mut stream : TcpStream) -> bool {
        loop {
            let mut req_buffer = BufReader::new(&stream);
            let mut resp_buffer = BufWriter::new(&stream);
            if   !self.parse_request(&mut req_buffer) || !self.parse_headers(&mut req_buffer)  {
                return false;
            }
                       
            if self.content_length > 0 {
                let content_length : usize = self.content_length;
                let mut buf = vec![0u8; self.content_length];
                req_buffer.read_exact(&mut buf);
                let data = std::str::from_utf8(&buf);
                match data {
                    Err(e) =>  panic!("Could not read data/body: {}", e) ,
                    Ok(t) =>  self.body = Some(t.to_string())
                }
            }

            self.add_header("Server", "CS3214-Personal-Server");
            let path = self.path.as_ref().unwrap();

            if !self.config.silent_mode {
                println!("< {:#?}", self);
            }
            
            if path.starts_with("/api") {
                self.handle_api(resp_buffer);
            }
            else if path.starts_with("/private") {
                println!("> {:#?}", self);
                if !self.verify_jwt() {
                    self.send_error(HttpPermissionDenied, "Permission denied. Please log in to access this resource.", resp_buffer);
                }
                else {
                    self.handle_static_asset(resp_buffer);
                }
            }
            else {
                self.handle_static_asset(resp_buffer);
            }

            /*
            let response = "HTTP/1.1 200 OK\r\n\r\n{}";
            stream.write( response.as_bytes() ).unwrap();
            stream.flush().unwrap();
            */
            if self.version == Http1_0 {
                return true;
            }
            //println!("{:#?}", self);
            //Reset the state of the Transaction 
            self.resp_body.clear();
            self.resp_headers.clear();
            self.req_headers.clear(); 
            self.cookies.clear();
            self.content_length = 0;
            self.range_start = 0;
            self.range_end = 0;
           
            self.resp_status = HttpInternalError;
            self.method = HttpUnknown;
            self.jwt = None;
            self.path = None; 
            self.body = None;
        }
        return false;
    }
}