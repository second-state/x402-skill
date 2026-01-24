mod cli;
mod config;
mod error;
mod output;
mod request;

use cli::Args;
use config::Config;
use error::X402Error;
use output::handle_response;
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

    // Build basic reqwest client (without x402 for now)
    let client_builder = reqwest::Client::builder();
    let client_builder = if req_config.follow_redirects {
        client_builder.redirect(reqwest::redirect::Policy::limited(10))
    } else {
        client_builder.redirect(reqwest::redirect::Policy::none())
    };
    let client = client_builder.build()?;

    // Build request
    let mut request = client.request(req_config.method, &req_config.url);
    request = request.headers(req_config.headers);
    if let Some(body) = req_config.body {
        request = request.body(body);
    }

    // Add basic auth if provided
    if let Some(user_pass) = &args.user {
        let parts: Vec<&str> = user_pass.splitn(2, ':').collect();
        let (user, pass) = if parts.len() == 2 {
            (parts[0], Some(parts[1]))
        } else {
            (parts[0], None)
        };
        request = request.basic_auth(user, pass);
    }

    // Send request
    let response = request.send().await?;

    // Handle response
    handle_response(
        response,
        args.output.as_deref(),
        args.fail,
        verbose,
    ).await?;

    Ok(())
}
