use super::{Command, Response};
use crate::{KVErrorKind, Result};
use futures::{SinkExt, StreamExt};
use tokio::net::{TcpStream, ToSocketAddrs};
use tokio_serde::formats::SymmetricalJson;
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

/// KvClient connects to a running [KvServer](crate::KvServer) through TCP and propagate user's
/// set/get/remove command to server, and show server's response after processing
/// the commands.
///
/// # Example:
///
/// See [Example in KvServer](crate::KvServer)
///
pub struct KvClient {
    stream: TcpStream,
}

impl KvClient {
    /// create a new client instance and connect to
    /// given server address
    pub async fn connect(addr: impl ToSocketAddrs) -> Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        Ok(Self { stream })
    }

    /// send a command to server and return the response
    /// from server
    pub async fn send(&mut self, command: Command) -> Result<Response> {
        let (read_half, write_half) = self.stream.split();
        let length_delimited = FramedWrite::new(write_half, LengthDelimitedCodec::new());

        let length_delimited_read = FramedRead::new(read_half, LengthDelimitedCodec::new());

        let mut deserialized: tokio_serde::Framed<_, Response, Response, _> =
            tokio_serde::SymmetricallyFramed::new(
                length_delimited_read,
                SymmetricalJson::<Response>::default(),
            );

        let mut serialized = tokio_serde::SymmetricallyFramed::new(
            length_delimited,
            SymmetricalJson::<Command>::default(),
        );

        serialized.send(command).await?;

        if let Some(res) = deserialized.next().await {
            Ok(res?)
        } else {
            Err(KVErrorKind::UnknownError.into())
        }
    }

    /// send a get command with key
    pub async fn send_get(&mut self, key: String) -> Result<Response> {
        self.send(Command::Get { key }).await
    }

    /// send a set command with key and val
    pub async fn send_set(&mut self, key: String, val: String) -> Result<Response> {
        self.send(Command::Set { key, val }).await
    }

    /// send a remove command with key
    pub async fn send_rm(&mut self, key: String) -> Result<Response> {
        self.send(Command::Remove { key }).await
    }
}
