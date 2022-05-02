use serde::Deserialize;
use serde_json::Deserializer;

use super::server::{Command, Response};
use crate::Result;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs, Shutdown};

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
        let reader = BufReader::new(&self.stream);
        let mut deserializer = Deserializer::from_reader(reader);
        let mut writer = BufWriter::new(&self.stream);

        // write command
        let command_bytes = serde_json::to_vec(&command)?;
        writer.write_all(&command_bytes[..])?;
        writer.flush()?;

        // read response
        let res: Response = Response::deserialize(&mut deserializer)?;
        Ok(res)
    }

    /// get the value of given key
    pub fn send_get(&mut self, key: String) -> Result<Response> {
        self.send(Command::Get { key })
    }

    /// set key to val
    pub fn send_set(&mut self, key: String, val: String) -> Result<Response> {
        self.send(Command::Set { key, val })
    }

    /// remove the given key
    pub fn send_rm(&mut self, key: String) -> Result<Response> {
        self.send(Command::Remove { key })
    }

    /// shutdown and drop the client
    pub fn shutdown(self) -> Result<()> {
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }
}
