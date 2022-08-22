use std::io;
use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::io::Read;
use std::io::Seek;
use std::io::SeekFrom;
use std::fs;
use std::fs::File;
use std::ffi::OsStr;
use std::path::Path;
use std::net::TcpStream;
use std::collections::HashMap;
use std::sync::Arc;

use crate::http::Method::*;
use crate::http::Version::*;
use crate::http::ResponseStatus::*;
use crate::credentials::Credentials;
use crate::server::Config;

use serde_json::Result;
use serde::{Deserialize, Serialize};
use jwt_simple::prelude::*;


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
    body: Option<String>,  //Request Body
    content_length: usize, 
    resp_status : ResponseStatus, 

    req_headers: HashMap<String, String>,

    cookies: HashMap<String, String>,
    resp_headers : Vec<String>, 
    resp_body : Vec<u8>, 

    jwt: Option<String>, 

    range_start : Option<u64>, 
    range_end: Option<u64>, 

    config: Arc<Config>
}


#[derive(Serialize, Deserialize)]
#[derive(Debug)]
struct Video {
    name : String, 
    size : u64
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
            range_start: None,
            range_end: None,
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
        let len = r.read_line(&mut line).unwrap();
        
        if len < 2 {
            return false;
        }
        line.pop();
        line.pop();
        
        let split : Vec<&str> = line.split(" ").collect();

        //This means we don't have the Method, the path, or the Version
        //Therefore we MUST close the connection
        if split.len() < 3 {
            return false;
        }

        if split[0] == "GET" {
            self.method = HttpGet;
        }
        else if split[0] == "POST" {
            self.method = HttpPost;
        }
        else {
            self.method = HttpUnknown;
        }

        self.path = Some(split[1].to_string());
        
        if split[2] == "HTTP/1.1" {
            self.version = Http1_1;
        }
        return true;
    }

    fn parse_headers<R: BufRead>(&mut self, r: &mut R) -> bool {
        for l in r.lines() {
            let line = l.unwrap();
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
                        let key = fields[0].trim().to_string();
                        let value = fields[1].trim().to_string();
                        if key == "auth" {
                            self.jwt = Some(value.clone());
                        }
                        self.cookies.insert(key, value);
                    }
                },
                "Range" => { 
                    let value : Vec<&str> = value.split("=").collect();
                    let value : Vec<&str> = value[1].split("-").collect();
                    let len = value.len();
                    match value[0].parse() {
                        Ok(v) => self.range_start = Some(v),
                        Err(_e) => {}
                    };

                    match value[1].parse() {
                        Ok(v) => self.range_end = Some(v),
                        Err(_e) => {}
                    };
                },
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
        self.resp_body.extend(message.as_bytes().to_vec());
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

    fn send_headers<W: Write>(&mut self, stream : &mut W) -> bool {
        let mut sb : Vec<String> = Vec::new();
        sb.push(self.start_response());
        sb.push(self.resp_headers.join(""));
        sb.push("\r\n".to_string());
        let headers : String = sb.join("");
        match stream.write(headers.as_bytes()) {
            Ok(_) => {return true;},
            Err(e) => {
                println!("Failed sending response: {}", e);
                return false;
            }
        }
    }

    fn send_body<W: Write>(&mut self, stream : &mut W) -> bool {
        let data : &[u8] = &self.resp_body.clone();
        match stream.write(data) {
            Ok(_) => {},
            Err(e) => { 
                println!("Failed sending response: {}", e);
                return false; 
            },
        }
        stream.flush().unwrap();
        return true;
    }

    fn send_response<W: Write>(&mut self, mut stream : W) -> bool {
        self.add_header("Content-Length", &self.resp_body.len().to_string());
        if !self.send_headers(&mut stream) {
            return false;
        }
        return self.send_body(&mut stream);
    }

    fn handle_api<W: Write>(&mut self, stream : W) {
        let path = self.path.as_ref().unwrap();
        if path == "/api/login" {
            self.handle_api_login(stream);
        }
        else if path == "/api/video" {
            let current_dir : String = self.config.server_root.clone();
            let walker = fs::read_dir(current_dir);
            match walker {
                Ok(scanner) => {
                    let mut videos : Vec<Video> = Vec::new();
                    for entry in scanner {
                        let entry = entry.unwrap();
                        let path = entry.path();
                        let extension = path.extension();
                        match extension {
                            None => {},
                            Some(ext) => {
                                if ext != "mp4" {
                                    continue;
                                }
                                let video : Video = Video{name: path.file_name().unwrap().to_str().unwrap().to_string(), size:  path.metadata().unwrap().len()};
                                videos.push(video);
                            }
                        }
                    }
                    self.add_header("Content-Type", "application/json");
                    self.resp_status = HttpOk;
                    self.resp_body.extend(serde_json::to_string(&videos).unwrap().as_bytes().to_vec());
                    self.send_response(stream);
                }
                Err(e) => {println!("Error when trying to scan directory: {}", e);}
            }
            
        }
        else {
            self.send_error(HttpNotFound, "API not implemented", stream);
        }
        
    }

    fn verify_jwt(&self) -> bool {
        let mut options = VerificationOptions::default();
        options.time_tolerance = Some(Duration::from_secs(0));
        match &self.jwt {
            None => return false,
            Some(jwt) => {
                let key = HS256Key::from_bytes( self.config.server_secret.as_bytes());
                let claims = key.verify_token::<NoCustomClaims>(&jwt, Some(options));
                match claims {
                    Ok(_c) =>  return true ,
                    Err(_e) => return false
                }
            }
        }
    }

    fn dump_jwt(&self, jwt : String) -> Option<String> {
        let key = HS256Key::from_bytes( self.config.server_secret.as_bytes());
        let claims = key.verify_token::<NoCustomClaims>(&jwt, None).unwrap();
        let json : String = format!("{{\"sub\": \"{}\", \"iat\": {}, \"exp\": {} }}", 
            claims.subject.unwrap(),
            claims.issued_at.unwrap().as_secs(),
            claims.expires_at.unwrap().as_secs()
        );
        return Some(json);
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
                let results = self.dump_jwt(v).unwrap();
                self.resp_body.extend( results.as_bytes().to_vec());

                return self.send_response(stream);
            },
            Err(e) => panic!("Could not generate JWT: {}", e)
        }
    }

    fn handle_api_login<W: Write>(&mut self, stream : W) -> bool {
        if self.method == HttpGet {
            self.add_header("Content-Type", "application/json");
            if self.verify_jwt() {
                let results = self.dump_jwt(self.jwt.as_ref().unwrap().to_string()).unwrap();
                self.resp_body.extend( results.as_bytes().to_vec());
                return self.send_response(stream);
            }
            else {
                self.resp_body.extend("{}".as_bytes().to_vec());
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

    fn handle_static_asset<W:Write>(&mut self, stream : &mut W) -> bool {
        let req_path : &str = self.path.as_ref().unwrap();

        //IDOR Redirection Attack Prevention
        if req_path.contains("..") {
            return self.send_error(HttpNotFound, "This file could not be found.", stream);
        }

        let file_name = format!("{}{}",self.config.server_root, req_path);
        let file_path = Path::new(&file_name);

        if file_path.exists() && file_path.is_file() {
            return self.send_file(file_path, stream);
        }

        let index_name = format!("{}{}",self.config.server_root, "/index.html");
        let index_path = Path::new(&index_name);
        if self.config.html5_fallback && index_path.exists() {
            return self.send_file(index_path, stream);
        }
        else {
            let message = format!("File {} not found", file_name);
            return self.send_error(HttpNotFound, &message, stream);
        }
    }

    fn guess_mime_type(&self, extension: Option<&OsStr> ) -> &str {
        match extension  {
            None => return "text/plain",
            Some(value) => {
                match value.to_str().unwrap() {
                    "html" => return "text/html",
                    "gif" => return "image/gif",
                    "png" => return "image/png",
                    "jpg" => return "image/jpeg",
                    "js" => return "text/javascript",
                    "mp4" => return "video/mp4",
                    "svg" => return "image/svg+xml",
                    "css" => return "text/css",
                    _ => return "text/plain"
                }
            }
        }
    }

    fn send_file<W:Write>(&mut self, path : &Path, stream : &mut W) -> bool {
        let file_size = path.metadata();
        let mut size : u64 = 0;
        let extension : Option<&OsStr> = path.extension();
        match file_size {
            Err(e) => {println!("Error: {}", e);},
            Ok(t) => {
                size = t.len();
            }
        }

        let mut partial_requested = false;
        //Compute the actual Ranges we should be sending 
        if let None = self.range_start {
            self.range_start = Some(0);
        }
        else {
            partial_requested = true;
        }

        if let None = self.range_end {
            self.range_end = Some(size);        
        }
        else {
            self.range_end = Some(self.range_end.unwrap() + 1);
        }



        let buffer_size : u64 = self.range_end.unwrap() - self.range_start.unwrap();
        
        let file =  File::open(path);

        self.add_header("Accept-Ranges", "bytes");
        let mime_type : String = self.guess_mime_type(extension).to_string();
        self.add_header("Content-Type", &mime_type);
        self.add_header("Content-Range", &format!("bytes {}-{}/{}",self.range_start.unwrap() , self.range_end.unwrap() - 1, size)[..]);
        self.add_header("Content-Length", &(buffer_size ).to_string());

        match file {
            Ok(f) => {
                let mut reader = BufReader::new(f);
                reader.seek(SeekFrom::Start(self.range_start.unwrap()));
                let mut reader = reader.take(buffer_size);
                
                self.resp_status = HttpOk;
                if partial_requested {
                    self.resp_status = HttpPartialContent;
                }
                self.send_headers(stream);
                let state = io::copy(&mut reader,stream);
                match state {
                    Ok(_s) => {},
                    Err(e) => println!("Failed to send data: {}", e)
                };
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




    pub fn http_handle_transaction(&mut self, stream : TcpStream) -> bool {
        loop {
            let mut req_buffer = BufReader::new(&stream);
            let mut resp_buffer = BufWriter::new(&stream);
            if   !self.parse_request(&mut req_buffer) || !self.parse_headers(&mut req_buffer)  {
                return false;
            }
                       
            if self.content_length > 0 {
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
            
            //println!("{:#?}", self);

            if path.starts_with("/api") {
                self.handle_api(resp_buffer);
            }
            else if path.starts_with("/private") {
                if !self.verify_jwt() {
                    self.send_error(HttpPermissionDenied, "Permission denied. Please log in to access this resource.", resp_buffer);
                }
                else {
                    self.handle_static_asset(&mut resp_buffer);
                }
            }
            else {
                self.handle_static_asset(&mut resp_buffer);
            }

            if self.version == Http1_0 {
                return true;
            }
            //Reset the state of the Transaction 
            self.resp_body.clear();
            self.resp_headers.clear();
            self.req_headers.clear(); 
            self.cookies.clear();
            self.content_length = 0;
            self.range_start = None;
            self.range_end = None;
           
            self.resp_status = HttpInternalError;
            self.method = HttpUnknown;
            self.jwt = None;
            self.path = None; 
            self.body = None;
        }
    }
}