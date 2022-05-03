# Road Map for PNA Rust Project 3

This is the implementation roadmap for project 3.



## KvServer & KvClient

The first challenge in this project is to establish a custom protocol over TCP used for communication between a `KvServer` and a `KvClient`. Fortunately, `serde`  and `serde_json` made it easy. 

Firstly, we define our communication data structure `Command` and `Response` as follows and derive `serde:{Serialize, Deserialize}` for them:

```Rust
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum Command {
    Get {
        key: String,
    },
    Set {
        key: String,
        val: String,
    },
    Remove {
        key: String,
    },
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Response {
    pub success: bool,
    pub message: String,
}
```

Since we have implemented `Serialize` for command, it is easy to send it over a `TcpStream` using following code:

```Rust
// TcpStream after connection established
let stream: TcpStream;
let command: Command;

let mut writer = BufReader::new(&stream);
// serialize a Command and flush it to writer
serde_json::to_writer(writer, &command);
```

The same code works for sending a `Response`, too. However, this does not translate directly to reading and deserializing a command/response. The following client code won't work:

```Rust
// TcpStream after connection established
let stream: TcpStream;

let mut reader = BufReader::new(&stream);
// this code will block waiting for an EOF from the other end of the stream
let response: Response = serde_json::from_reader(reader);
```

When calling `from_reader`, `serde_json` expects to consume the reading end it receives(receiving an `EOF` in the context of a TCP connection), but the server, after writng the first response, is also waitng on its reading end to read more command from the client, and hence would not send further response or `EOF` through its writing end. Therefore, the client and the server will deadlock, waiting for each other.

This is where a protocol comes into play. Currently my design is to let the client initiate connection, send command and end connection by calling `shutdown`. On the other hand, the server, once a connection is established, continuously waits for new command from the client and send response correspondingly. The server ends the connection only after the client tells it to. Translating it into code, the client and server's send/receive functions are implemented as follows:

- Client End:

```Rust
// client's send function
pub fn send(&mut self, command: Command) -> Result<Response> {
        let reader = BufReader::new(&self.stream);
        let mut deserializer = Deserializer::from_reader(reader);
        let mut writer = BufWriter::new(&self.stream);

        // write command
        let command_bytes = serde_json::to_vec(&command)?;
        writer.write_all(&command_bytes[..])?;
        writer.flush()?;

        // read response
  			// client doesn't read to the end of the stream, rather, it tries 
  			// to deserialize just one response and proceed
        let res: Response = Response::deserialize(&mut deserializer)?;
        Ok(res)
}

// shutdown the tcp channel and drop the client
// the method will close the tcp stream, causing the server's iterator
// to terminate
pub fn shutdown(self) -> Result<()> {
    		self.stream.shutdown(Shutdown::Both)?;
    		Ok(())
}
```

- Server End:

```Rust
fn handle_connection(&mut self, stream: TcpStream) -> Result<()> {
        let reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);
        // interpret the stream as a sequence of Command types
        let command_reader = Deserializer::from_reader(reader);   
  			// server waits for commands indefinitely, and only stop and return
  			// when the command_reader is closed by client
        for command in command_reader.into_iter() {
            // deserialize a command
            let command = command?;
          
            // process command to a response
            let response = self.process_command(command);

            // write response
            let response_bytes = serde_json::to_vec(&response)?;
            writer.write_all(&response_bytes[..])?;
            writer.flush()?;
        }
        Ok(())
}
```

In this way, communication can proceed without blocking.