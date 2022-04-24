use super::protocol;
use super::server::{Command, Response};
use crate::Result;
use std::io::{BufReader, BufWriter};
use std::net::{SocketAddr};
use futures::SinkExt;
use tokio::net::{TcpStream};
use tokio_serde::formats::SymmetricalJson;
use tokio_util::codec::{FramedWrite, LengthDelimitedCodec};

/// KvClient structure that handles
/// communication with server
pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    /// create a new client instance and connect to
    /// given server address
    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    /// send a command to server and return the response
    /// from server
    pub async fn send(&mut self, command: Command) -> Result<()> {
        let (read_half, write_half) = self.stream.split();
        let length_delimited = FramedWrite::new(
            write_half, 
            LengthDelimitedCodec::new()
        );

        let mut serialized =
            tokio_serde::SymmetricallyFramed::new(
                length_delimited, 
                SymmetricalJson::<Command>::default()
            );

        serialized.send(command).await?;

        Ok(())
    }

    // /// send a get command with key
    // pub fn sent_get(&mut self, key: String) -> Result<Response> {
    //     self.send(Command::Get { key })
    // }

    // /// send a set command with key and val
    // pub fn send_set(&mut self, key: String, val: String) -> Result<Response> {
    //     self.send(Command::Set { key, val })
    // }

    // /// send a remove command with key
    // pub fn send_rm(&mut self, key: String) -> Result<Response> {
    //     self.send(Command::Remove { key })
    // }

}
