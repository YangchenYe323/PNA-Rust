use clap::{Parser, Subcommand};
use kvs::{Command, KvClient, Response};
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    process::exit,
};

#[derive(Parser, Debug)]
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

#[derive(Subcommand, Debug)]
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

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let command = match args.command {
        SubCommand::Get { key } => Command::Get { key },

        SubCommand::Set { key, val } => Command::Set { key, val },

        SubCommand::Rm { key } => Command::Remove { key },
    };

    let mut client = KvClient::connect(args.addr).await.expect("Fail to create connection");

    let response = client.send(command).await.expect("Fail to receive response");

    // match response {
    //     Response {
    //         success: true,
    //         message,
    //     } => {
    //         if !message.is_empty() {
    //             println!("{}", message);
    //         }
    //         exit(0);
    //     }
    //     Response {
    //         success: false,
    //         message,
    //     } => {
    //         eprintln!("{}", message);
    //         exit(1);
    //     }
    // }
}