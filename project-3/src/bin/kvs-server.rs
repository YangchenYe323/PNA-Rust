use clap::{ Parser };
use std::net::{ SocketAddr, IpAddr, Ipv4Addr };

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
	#[clap(help = "KV Engine used by server")]
	engine: Option<Engine>,
}

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

fn main() {
	Args::parse();
}