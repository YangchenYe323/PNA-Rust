use serde_json::Deserializer;
use serde::Deserialize;
use super::{Command, Response};
use crate::Result;
use std::io::{BufReader, BufWriter, Write};
use std::net::{TcpStream, ToSocketAddrs, Shutdown};

/// KvClient connects to a running [KvServer](crate::KvServer) through TCP and propagate user's
/// set/get/remove command to server, and show server's response after processing
/// the commands.
/// 
pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    /// create a new client instance and connect to
    /// given server address
    /// 
    /// # Error
    /// propagate I/O error by by [TcpStream](TcpStream::connect)
    pub fn new(addr: impl ToSocketAddrs) -> Result<Self> {
        let stream = TcpStream::connect(addr)?;
        Ok(Self { stream })
    }

    /// send a command to server and return the response
    /// from server
    /// 
    /// # Error
    /// Network I/O error that causes failure to send or receive message.
    /// (Note): KvStore's error are indicated by [success field in Response](crate::Response)
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

    /// wrapper around [send](crate::KvClient::send) get the value of given key
    pub fn send_get(&mut self, key: String) -> Result<Response> {
        self.send(Command::Get { key })
    }

    /// wrapper around [send](crate::KvClient::send) set key to val
    pub fn send_set(&mut self, key: String, val: String) -> Result<Response> {
        self.send(Command::Set { key, val })
    }

    /// wrapper around [send](crate::KvClient::send) remove the given key
    pub fn send_rm(&mut self, key: String) -> Result<Response> {
        self.send(Command::Remove { key })
    }

    /// shutdown the tcp channel and drop the client
    pub fn shutdown(self) -> Result<()> {
        self.stream.shutdown(Shutdown::Both)?;
        Ok(())
    }
}
