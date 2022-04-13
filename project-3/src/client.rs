use std::net::{ ToSocketAddrs, TcpStream };
use std::io::{ BufReader, BufWriter };
use super::server::{ Command, Response };
use super::protocol;
use crate::Result;

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
		Ok(Self {
			stream
		})
	}

	/// send a command to server and return the response
	/// from server
	pub fn send(&mut self, command: Command) -> Result<Response> {
		let mut reader = BufReader::new(&self.stream);
		let mut writer = BufWriter::new(&self.stream);

		protocol::write(writer, command)?;

		let res: Response = protocol::read(reader)?;
		Ok(res)
	}

}