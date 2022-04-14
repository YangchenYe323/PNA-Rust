use clap::Parser;
use std::fmt;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::fs::{OpenOptions};
use std::io::{Read, Write};
use std::process::exit;
use tracing::{info, Level};
use kvs::{ KvServer, KvsEngine, KvStore };

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(long)]
    #[clap(
        default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4000))
    ]
    #[clap(help = "Socket Address to bind this server to")]
    addr: SocketAddr,

    #[clap(long)]
    #[clap(help = "KV Engine used by server")]
    engine: Option<Engine>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Engine {
    Kvs,
    Sled,
}

impl Engine {
    fn parse(s: String) -> Result<Option<Self>, String> {
        if s.is_empty() {
            Ok(None)
        } else if s == String::from("kvs") {
            Ok(Some(Engine::Kvs))
        } else if s == String::from("sled") {
            Ok(Some(Engine::Sled))
        } else {
            Err(String::from("Unknown engine type"))
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        let s = match self {
            Engine::Kvs => "kvs",
            Engine::Sled => "sled",
        };
        Vec::from(s.as_bytes())
    }
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

    info!("Application Started: Version {}", env!("CARGO_PKG_VERSION"));

    let engine = create_storage(args.engine);

    let server = KvServer::new(args.addr, engine).unwrap();

    info!("Server started listening to {}", args.addr);

	server.run();
}

fn create_storage(kind: Option<Engine>) -> impl KvsEngine {

    let dirpath = std::env::current_dir().unwrap();
    
    let metadata_path = dirpath.join("metadata");
    let mut metadata_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&metadata_path).expect("Cannot open metadata");

    let mut content = String::new();
    metadata_file.read_to_string(&mut content).expect("Cannot read metadata");
    let preset_engine: Option<Engine> = Engine::parse(content).expect("Metadata format error");

    let final_engine = if preset_engine.is_none() {
        let new_kind = if kind.is_none() {
            Engine::Kvs
        } else {
            kind.unwrap()
        };
        let bytes = new_kind.to_bytes();
        metadata_file.write(&bytes[..]).unwrap();
        metadata_file.flush().unwrap();
        Some(new_kind)
    } else {
        if let None = kind {
            Some(preset_engine.unwrap())
        } else {
            let default_kind = preset_engine.unwrap();
            let selected_kind = kind.unwrap();
            if default_kind == selected_kind {
                Some(default_kind)
            } else {
                None
            }
        }
    };

    if final_engine.is_none() {
        eprintln!("Wrong Engine");
        exit(1);
    }

    let final_engine = final_engine.unwrap();
    info!("Application use storage engine: {}", final_engine);

    match final_engine {
        Engine::Kvs => {
            KvStore::open(&dirpath).unwrap()
        }

        Engine::Sled => {
            KvStore::open(&dirpath).unwrap()
        }
    }
}
