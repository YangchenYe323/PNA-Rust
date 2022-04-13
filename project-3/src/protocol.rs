use std::net::TcpStream;
use std::io::{ Cursor, BufReader, BufWriter, Read, Write };
use serde::{ Serialize, Deserialize };
use serde::de::DeserializeOwned;
use byteorder::{ ReadBytesExt, WriteBytesExt, NetworkEndian };
use crate::Result;

pub(crate) fn read<T>(mut reader: BufReader<&TcpStream>) -> Result<T>
	where T: DeserializeOwned
{
	// read and parse the length of the structure
	let mut length_buffer: [u8; 8] = [0; 8];
	let l = reader.read(&mut length_buffer)?;
	let length = Cursor::new(length_buffer.to_vec())
		.read_u64::<NetworkEndian>()?;

	// read and parse structure
	let mut structure_reader = reader.take(length);
	let structure: T = serde_json::from_reader(&mut structure_reader)?;

	Ok(structure)
}

pub(crate) fn write<T>(
	mut writer: BufWriter<&TcpStream>, 
	content: T
) -> Result<()>
	where T: Serialize
{
	let content_bytes = serde_json::to_vec(&content)?;
	let length = content_bytes.len() as u64;
	let mut length_bytes = vec![];
	length_bytes.write_u64::<NetworkEndian>(length)?;

	// send length
	writer.write(&length_bytes[..])?;
	// send content
	writer.write(&content_bytes[..])?;
	writer.flush()?;

	Ok(())
}