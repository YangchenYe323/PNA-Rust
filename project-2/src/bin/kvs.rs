use clap::{Parser, Subcommand};
use kvs::KvStore;
use std::process::exit;
#[derive(Parser)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: SubCommand,
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

    let pwd = std::env::current_dir().expect("Cannot Open Working Directory");
    let mut kv = KvStore::open(&pwd).expect("Failed to Open Store");

    match args.command {
        SubCommand::Get { key } => {
            let result = kv.get(key);
            match result {
                Ok(val) => {
                    if let Some(string_val) = val {
                        println!("{}", string_val);
                    } else {
                        println!("Key not found");
                    }
                    exit(0);
                }

                Err(error) => {
                    eprintln!("{}", error);
                    exit(1);
                }
            }
        }

        SubCommand::Set { key, val } => {
            let result = kv.set(key, val);
            match result {
                Ok(_) => exit(0),
                Err(error) => {
                    eprintln!("{}", error);
                    exit(1);
                }
            }
        }

        SubCommand::Rm { key } => {
            let result = kv.remove(key);
            match result {
                Ok(_) => exit(0),

                Err(error) => {
                    println!("{}", error);
                    exit(1);
                }
            }
        }
    }
}
