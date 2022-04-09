use clap::Parser;
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
	// operation to perform on database
	op: Option<String>,

	// key of operation
	key: Option<String>,

	// value of operation
	value: Option<String>,

}

fn main() -> Result<(), String> {
	let args = Args::parse();

	match args.op {
		Some(_) => {
			Err(String::from("unimplemented"))
		}

		None => {
			Err(String::from("No Op provided"))
		}
	}
}