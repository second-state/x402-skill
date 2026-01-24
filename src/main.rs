mod cli;
mod error;

use cli::Args;
use error::X402Error;
use std::process::ExitCode;

#[tokio::main]
async fn main() -> ExitCode {
    match run().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error [{}]: {}", e.code_number(), e);
            e.exit_code()
        }
    }
}

async fn run() -> Result<(), X402Error> {
    let args = Args::parse_args();

    if args.verbose {
        eprintln!("> {} {}", args.method, args.url);
    }

    println!("URL: {}", args.url);
    println!("Method: {}", args.method);
    Ok(())
}
