use super::protocol;
use super::server::{Command, Response};
use crate::Result;
use std::io::{BufReader, BufWriter};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};

/// KvClient structure that handles
/// communication with server
pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    /// create a new client instance and connect to
    /// given server address
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }

    /// send a command to server and return the response
    /// from server
    pub fn send(&mut self, command: Command) -> Result<Response> {
        let mut reader = BufReader::new(&self.stream);
        let mut writer = BufWriter::new(&self.stream);

        protocol::write(&mut writer, command)?;

        let res: Response = protocol::read(&mut reader)?;
        Ok(res)
    }

    /// send a get command with key
    pub fn sent_get(&mut self, key: String) -> Result<Response> {
        self.send(Command::Get { key })
    }

    /// send a set command with key and val
    pub fn send_set(&mut self, key: String, val: String) -> Result<Response> {
        self.send(Command::Set { key, val })
    }

    /// send a remove command with key
    pub fn send_rm(&mut self, key: String) -> Result<Response> {
        self.send(Command::Remove { key })
    }

    /// shutdown the client end of TCP
    pub fn shutdown(self) -> Result<()> {
        Ok(self.stream.shutdown(Shutdown::Both)?)
    }
}
