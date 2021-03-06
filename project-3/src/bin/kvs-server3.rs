use clap::Parser;
use kvs_project_3::{KvServer, KvStore, KvsEngine, SledKvsEngine};
use std::fmt;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;
use std::process::exit;
use tracing::{info, Level};

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
        } else if s == *"kvs" {
            Ok(Some(Engine::Kvs))
        } else if s == *"sled" {
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
    info!("Application Version {}", env!("CARGO_PKG_VERSION"));

    let args = Args::parse();

    // create engine based on command line argument and
    // metadata file
    let engine = create_storage(args.engine);

    let server = KvServer::new(args.addr, engine).unwrap();

    info!("Server started listening to {}", args.addr);

    server.run();
}

fn create_storage(kind: Option<Engine>) -> Box<dyn KvsEngine> {
    // we store the kind of engine we run with in
    // "./medadata"
    let dirpath = std::env::current_dir().unwrap();
    let metadata_path = dirpath.join("metadata");
    let mut metadata_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(&metadata_path)
        .expect("Cannot open metadata");

    // parse existing metadata
    let mut content = String::new();
    metadata_file
        .read_to_string(&mut content)
        .expect("Cannot read metadata");
    let preset_engine: Option<Engine> = Engine::parse(content).expect("Metadata format error");

    let final_engine = if let Some(default_kind) = preset_engine {
        // here engine is set by metadata
        // we only proceed if command-line specified engine
        // matches it
        if let Some(selected_kind) = kind {
            // here commandline engine is set and match
            if default_kind == selected_kind {
                Some(default_kind)
            } else {
                // command-line engine is set and doesn't match
                None
            }
        } else {
            // command-line engine option is not set
            // use the engine from metadata
            Some(default_kind)
        }
    } else {
        // metdata is not set
        // get engine from command-line or Kvs as default
        let new_kind = kind.unwrap_or(Engine::Kvs);
        // save this choice to metadata
        let bytes = new_kind.to_bytes();
        metadata_file.write_all(&bytes[..]).unwrap();
        metadata_file.flush().unwrap();
        Some(new_kind)
    };

    if final_engine.is_none() {
        eprintln!("Wrong Engine");
        exit(1);
    }

    let final_engine = final_engine.unwrap();
    info!("Application use storage engine: {}", final_engine);

    match final_engine {
        Engine::Kvs => Box::new(open_kvs(dirpath)),

        Engine::Sled => Box::new(open_sled(dirpath)),
    }
}

fn open_kvs(dirpath: PathBuf) -> impl KvsEngine {
    KvStore::open(&dirpath).unwrap()
}

fn open_sled(dirpath: PathBuf) -> impl KvsEngine {
    SledKvsEngine::open(dirpath).unwrap()
}
