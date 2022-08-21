use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[derive(Debug)]
pub struct Credentials {
    pub username : String, 
    pub password : String 
}

impl Credentials {
    pub fn valid(&self) -> bool {
        return self.username == "user0" && self.password == "thepassword";
    }
}