use clap::Parser;
use std::fmt;
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use tracing::{debug, info, trace, Level};
use kvs::KvServer;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    #[clap(
        default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4000))
    ]
    #[clap(help = "Socket Address to bind this server to")]
    addr: SocketAddr,

    #[clap(long)]
    #[clap(default_value_t = Engine::Kvs)]
    #[clap(help = "KV Engine used by server")]
    engine: Engine,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Engine {
    Kvs,
    Sled,
}

impl std::str::FromStr for Engine {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s == "kvs" {
            Ok(Self::Kvs)
        } else if s == "sled" {
            Ok(Self::Sled)
        } else {
            Err(Self::Err::from("Unsupported KV Engine"))
        }
    }
}

impl fmt::Display for Engine {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::Kvs => write!(f, "kvs"),
            Self::Sled => write!(f, "sled"),
        }
    }
}

fn main() {
    // set log collector
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .pretty()
        .with_max_level(Level::DEBUG)
        .init();

    info!("Logger Initialized");

    let args = Args::parse();

    info!(
        "Listening on {}, Using storage engine {:?}",
        args.addr, args.engine
    );

    let server = KvServer::new(args.addr).unwrap();
	server.run().unwrap();
}

fn handle_client(mut stream: TcpStream) {
    debug!("Accepted connection: {:?}", &stream);
}
