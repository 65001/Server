use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Credentials {
    username : String, 
    password : String 
}

impl Credentials {
    pub fn valid(&self) -> bool {
        return self.username == "user0" && self.password == "thepassword";
    }
}