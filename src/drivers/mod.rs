pub mod linux;
use std::fmt;
use std::error::Error;

#[derive(Clone, PartialEq, Debug)]
pub enum Mode {
    User,
    Superuser,
    Config,
    Undefined
}

#[derive(Debug)]
pub struct NetrukoError {
    caused_by: Box<str>
}

impl fmt::Display for NetrukoError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.caused_by)
    }
}

impl Error for NetrukoError {
    fn description(&self) -> &str {
        &(*self.caused_by)
    }
}

impl NetrukoError {
    pub fn new(new_cause: Box<str>) -> Self {
        Self{caused_by: new_cause}
    }
}

pub trait NetrukoDriver {
    fn get_mode(&self) -> Mode;
    fn set_mode(&mut self, mode: Mode);
    fn get_input(&mut self, input: &mut russh::CryptoVec) -> Result<Vec<String>, Box<dyn Error>>;
    fn do_become(&mut self) -> (String, String);
    fn can_become(self) -> bool;
    fn can_config(self) -> bool;
    fn do_config(self) -> (String, String);
}
