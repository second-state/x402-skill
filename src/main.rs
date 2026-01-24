mod error;

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
    println!("x402curl - coming soon");
    Ok(())
}
