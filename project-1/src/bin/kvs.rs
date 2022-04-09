use clap::{Parser, Subcommand};

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

fn main() -> Result<(), String> {
    let args = Args::parse();

    match args.command {
        SubCommand::Get { key: _key } => Err("unimplemented".to_owned()),

        SubCommand::Set {
            key: _key,
            val: _val,
        } => Err("unimplemented".to_owned()),

        SubCommand::Rm { key: _key } => Err("unimplemented".to_owned()),
    }
}
