mod cli;
mod config;
mod error;
mod request;

use cli::Args;
use config::Config;
use error::X402Error;
use request::RequestConfig;
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
    let config = Config::load(args.x402_key.as_deref())?;
    let req_config = RequestConfig::from_args(&args)?;

    let verbose = args.verbose || config.verbose;

    if verbose {
        eprintln!("> {} {}", req_config.method, req_config.url);
        for (name, value) in req_config.headers.iter() {
            eprintln!("> {}: {}", name, value.to_str().unwrap_or("<binary>"));
        }
    }

    if !args.x402_dry_run {
        let key = config.require_private_key()?;
        if verbose {
            let masked = if key.len() > 10 {
                format!("{}...{}", &key[..6], &key[key.len()-4..])
            } else {
                "***".to_string()
            };
            eprintln!("* Using key: {}", masked);
        }
    }

    Ok(())
}
