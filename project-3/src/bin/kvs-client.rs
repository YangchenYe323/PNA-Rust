use clap::{Parser, Subcommand};
use kvs::KvStore;
use kvs::Command;
use kvs::Response;
use byteorder::{ ReadBytesExt, WriteBytesExt, NetworkEndian };
use std::net::{ IpAddr, Ipv4Addr, SocketAddr, TcpStream };
use std::io::{ Cursor, BufReader, BufWriter, Write, Read };
use std::process::exit;
use kvs::KvClient;

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

    let command = match args.command {
        SubCommand::Get { key } => {
            Command::Get {
                key,
            }
        }

        SubCommand::Set { key, val } => {
            Command::Set {
                key, val,
            }
        }

        SubCommand::Rm { key } => {
            Command::Remove {
                key
            }
        }
    };

    let mut client = KvClient::new(args.addr).expect("Fail to create connection");

    let response = client.send(command).expect("Fail to receive response");
    
    println!("{:?}", response);

}
