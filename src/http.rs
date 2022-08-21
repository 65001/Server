use std::io::BufRead;
use std::io::BufReader;
use std::io::BufWriter;
use std::io::Write;
use std::net::TcpStream;

use std::collections::HashMap;

use crate::http::Method::*;
use crate::http::Version::*;
use crate::http::ResponseStatus::*;

#[derive(Debug)]
enum Method {
    HttpPost,
    HttpGet,
    HttpUnknown
}

#[derive(Debug)]
enum Version {
    Http1_0,
    Http1_1
}

#[derive(Debug)]
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
    content_length: u32, 
    http_response_status : ResponseStatus, 

    req_headers: HashMap<String, String>,

    resp_headers : Vec< (String, String) >, 
    resp_body : Option<String>, 

    jwt: Option<String>, 

    range_start :u32, 
    range_end: u32, 
}

impl Transaction {
    pub fn new() -> Self {
        Self {
            method : HttpUnknown,
            version : Http1_0,
            http_response_status : HttpInternalError, 
            jwt: None,
            body: None,
            path: None,
            range_start: 0,
            range_end: 0,
            content_length: 0,
            resp_body: None,
            resp_headers: Vec::new(),
            req_headers: HashMap::new(),
        }
    }

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
            println!("SPLIT: {:?}, {:?}", split, line);
            return false;
        }

        if split[0] == "GET" {
            self.method = HttpGet;
        }
        else if split[0] == "POST" {
            self.method = HttpPost;
        }
        else {
            println!("SPLIT: {:?}", split);
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
            println!("{}", line);
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
                "Cookie" => {},
                "Range" => {},
                _ => {}
            }

            self.req_headers.insert( key.to_string(), value );

            

            
        }
        return true;
    }

    pub fn http_handle_transaction(&mut self, mut stream : TcpStream) -> bool {
        let mut req_buffer = BufReader::new(&stream);
        if !self.parse_request(&mut req_buffer) || !self.parse_headers(&mut req_buffer) {
            return false;
        }

        //Load the HTTP REQ_BODY 

        let response = "HTTP/1.1 200 OK\r\n\r\n{}";
        stream.write( response.as_bytes() ).unwrap();
        stream.flush().unwrap();
        

        println!("{:#?}", self);

        return false;
    }
}