use clap::{Parser, Subcommand};
use kvs::KvStore;
use std::net::{ IpAddr, Ipv4Addr, SocketAddr, TcpStream };
use std::process::exit;

#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: SubCommand,

    #[clap(long)]
    #[clap(
        default_value_t = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4000))
    ]
    #[clap(help = "Server Address")]
    addr: SocketAddr,
}

#[derive(Subcommand)]
enum SubCommand {
    #[clap(about = "Get string value of a given string key")]
    Get {
        #[clap(help = "The string key")]
        key: String,
    },

    #[clap(about = "Set string value of a given string key")]
    Set {
        #[clap(help = "The string key")]
        key: String,
        #[clap(help = "The value assigned to key")]
        val: String,
    },

    #[clap(about = "Remove a given key")]
    Rm {
        #[clap(help = "The string key to remove")]
        key: String,
    },
}

fn main() {
    let args = Args::parse();
    println!("{:?}", args.addr);

    let connection = TcpStream::connect(args.addr).unwrap();

    match args.command {
        SubCommand::Get { key } => {}

        SubCommand::Set { key, val } => {}

        SubCommand::Rm { key } => {}
    }
}
