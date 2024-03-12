pub mod drivers;

use std::error::Error;
use russh;
use russh_keys;
use async_trait::async_trait;
use crate::drivers::{NetrukoDriver, NetrukoError, Mode, linux::Linux};

struct Client {}

#[async_trait]
impl russh::client::Handler for Client {
    type Error = russh::Error;
    async fn check_server_key(
        self,
        _server_public_key: &russh_keys::key::PublicKey,
    ) -> Result<(Self, bool), Self::Error> {
        Ok((self, true))
    }
}

pub struct Netruko {
    host: String,
    login: String,
    password: String,
    become_password: Option<String>,
    driver: Box<dyn NetrukoDriver>,
    session: Option<russh::client::Handle<Client>>,
    vty: Option<russh::Channel<russh::client::Msg>>
}

impl Netruko {
    pub fn new(host: &str, driver: &str, username: &str, password: &str, become_password: Option<&str>) -> Option< Self > {
        match driver.to_lowercase().as_str() {
            "linux" => {
                Some(
                    Self{
                        host: host.to_string(),
                        login: username.to_string(),
                        password: password.to_string(),
                        become_password: become_password.map(String::from),
                        driver: Box::new(Linux::new()),
                        session: None,
                        vty: None
                    }
                )
            },
            _ => {None}
        }
    }
    pub async fn connect(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let config = russh::client::Config::default();
        let handler = Client{};
        let mut session = russh::client::connect(config.into(), &self.host, handler).await?;
        if session.authenticate_password(&self.login, &self.password).await.unwrap() {
            let vty = session.channel_open_session().await?;
            vty.request_pty(false, "vt100", 80, 25, 0, 0, &[]).await?;
            vty.request_shell(true).await?;
            self.vty = Some(vty);
            while self.driver.get_mode() == Mode::Undefined {
                self.read().await?;
            };
            self.session = Some(session);
            return Ok(())
        } else {
            return Err(Box::new(russh::Error::NotAuthenticated))
        }
    }
    async fn read(&mut self) -> Result<Vec<String>, Box<dyn Error>> {
        if let Some(ref mut vty) = self.vty {
            loop {
                if let Some(mut msg) = vty.wait().await {
                    match msg {
                        russh::ChannelMsg::Data { ref mut data } => {
                            return self.driver.get_input(data)
                        },
                        russh::ChannelMsg::Eof => {
                        },
                        russh::ChannelMsg::Close => {
                        }
                        _ => {
                        }
                    }
                } else {
                    break;
                }
            }
        }
        Ok(Vec::new())
    }
    async fn send(&mut self, buf: &[u8]) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut vty) = self.vty {
            vty.data(buf).await?;
        } else {
            return Err(Box::new(russh::Error::Disconnect));
        }
        Ok(())
    }
    pub async fn disconnect(&mut self) -> Result<(), Box<dyn Error>> {
        if let Some(ref mut vty) = self.vty {
            vty.close().await?;
            self.vty = None
        }
        if let Some(ref mut session) = self.session {
            session.disconnect(russh::Disconnect::ByApplication, "", "English").await?;
            self.session = None;
        }
        Ok(())
    }
    pub async fn command(&mut self, cmd: String, need_become: bool) -> Result<Vec<String>, Box<dyn Error>> {
        if need_become {
            self.do_become().await?;
        }
        self.driver.set_mode(Mode::Undefined);
        let data = cmd.clone() + "\n";
        self.send(data.as_bytes()).await?;
        let mut result: Vec<String> = Vec::new();
        while self.driver.get_mode() == Mode::Undefined {
            result.extend_from_slice(&self.read().await?);
        }
        if result.len() > 1 {
            result.remove(0);
        }
        return Ok(result);
    }
    pub async fn do_become(&mut self) -> Result<(), Box<dyn Error>> {
        if self.become_password.is_none() {
            return Err(Box::new(russh::Error::Inconsistent));
        }
        let (become_cmd, become_prompt) = self.driver.do_become();
        let mut data = become_cmd;
        data.push('\n');
        self.send(data.as_bytes()).await?;
        self.driver.set_mode(Mode::Undefined);
        while self.driver.get_mode() == Mode::Undefined {
            for line in self.read().await? {
                if line == become_prompt {
                    data = self.become_password.clone().unwrap();
                    data.push('\n');
                    self.send(data.as_bytes()).await?;
                }
            }
        }
        if self.driver.get_mode() != Mode::Superuser {
            return Err(Box::new(NetrukoError::new("Failed to become".into())));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
