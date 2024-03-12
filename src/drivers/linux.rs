use std::error::Error;
use std::io::Write;
use std::str;
use crate::drivers::{Mode, NetrukoDriver};

pub struct Linux {
    mode: Mode,
}

impl Linux {
    const USER: char = '$';
    const SUPERUSER: char = '#';

    pub fn new() -> Self {
        Self {
            mode: Mode::Undefined
        }
    }
    fn parse_string(&self, input: String) -> String {
        let mut escape: bool = false;
        let mut csi: bool = false;
        let mut result: String = String::new();
        for subs in input.split("") {
            if subs.len() == 0 {
                continue
            }
            match subs.as_bytes()[0] {
                0x1b => {
                    escape = true;
                },
                0x5b => {
                    if escape && !csi {
                        csi = true;
                        escape = false;
                    } else if csi {
                        csi = false;
                        escape = false;
                    } else {
                        result.push_str(subs);
                    }
                }
                0x40..=0x7e => {
                    if csi {
                        csi = false;
                    } else {
                        result.push_str(subs);
                    }
                },
                _ => {
                    if !csi {
                        result.push_str(subs);
                    }
                }
            }
        }
        return result;
    }
}
impl NetrukoDriver for Linux {
    fn get_mode(&self) -> Mode {
        self.mode.clone()
    }
    fn set_mode(&mut self, mode: Mode) {
        self.mode = mode;
    }
    fn get_input(&mut self, input: &mut russh::CryptoVec) -> Result<Vec<String>,Box<dyn Error>> {
        let mut buffer: Vec<u8> = Vec::new();
        buffer.write_all(input)?;
        let mut result:Vec<String> = str::from_utf8(&buffer)?
                                    .trim()
                                    .lines()
                                    .map(|x| self.parse_string(x.to_string()).trim().to_string())
                                    .filter(|x| x.len() > 0)
                                    .collect();
        if !result.is_empty() {
            let input = result.pop().unwrap();
            if let Some(delimiter) = input.chars().position(|x| x == Linux::USER || x == Linux::SUPERUSER) {
                match input.chars().nth(delimiter).unwrap() {
                    Linux::USER => {
                        self.mode = Mode::User;
                    },
                    Linux::SUPERUSER => {
                        self.mode = Mode::Superuser;
                    },
                    _ => {result.push(input)}
                }
            } else {
                result.push(input);
            }
        }
        Ok(result)
    }
    fn do_become(&mut self) -> (String, String) {
        (String::from("su"), String::from("Password:"))
    }
    fn can_become(self) -> bool {
        true
    }
    fn can_config(self) -> bool {
        false
    }
    fn do_config(self) -> (String, String) {
        (String::new(), String::new())
    }
}

#[cfg(test)]
mod tests {
    use super::Linux;
    use crate::drivers::{Mode, NetrukoDriver};
    use russh::CryptoVec;

    #[test]
    fn mode_selection() {
        let mut driver = Linux::new();
        let mut buffer = CryptoVec::from_slice(b"This is an initial prompt\n\rlocalhost:~$");
        let _ = driver.get_input(&mut buffer);
        assert_eq!(driver.get_mode(), Mode::User);
        let mut buffer = CryptoVec::from_slice(b"This is an initial prompt\n\rlocalhost:~#");
        let _ = driver.get_input(&mut buffer);
        assert_eq!(driver.get_mode(), Mode::Superuser);
    }
    #[test]
    fn escape_sequences() {
        let mut driver = Linux::new();
        let mut buffer = CryptoVec::from_slice("This is an initial prompt\u{001b}[2001l unicode\n\rlocalhost:~$".as_bytes());
        let result = driver.get_input(&mut buffer).unwrap();
        assert_eq!(result[0], String::from("This is an initial prompt unicode"));
    }
}
