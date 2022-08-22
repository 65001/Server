use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Credentials {
    pub username : String, 
    pub password : String 
}


impl Credentials {
    /*
    If this wasn't just a demo server, this could go out to the database and verify 
    the credentials. 
    */
    pub fn valid(&self) -> bool {
        return self.username == "user0" && self.password == "thepassword";
    }
}