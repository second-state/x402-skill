mod cli;
mod config;
mod error;
mod output;
mod request;

use alloy_signer_local::PrivateKeySigner;
use cli::Args;
use config::Config;
use error::X402Error;
use output::handle_response;
use request::RequestConfig;
use reqwest_middleware::ClientWithMiddleware;
use std::process::ExitCode;
use std::sync::Arc;
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};
use x402_rs::scheme::v1_eip155_exact::client::V1Eip155ExactClient;
use x402_rs::scheme::v2_eip155_exact::client::V2Eip155ExactClient;

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

    // Get private key and build x402 client
    let private_key = config.require_private_key()?;
    let signer: PrivateKeySigner = private_key
        .parse()
        .map_err(|e| X402Error::Config(format!("Invalid private key: {}", e)))?;

    if verbose {
        eprintln!("* Signing address: {:?}", signer.address());
    }

    let signer = Arc::new(signer);
    let x402_client = X402Client::new()
        .register(V1Eip155ExactClient::new(signer.clone()))
        .register(V2Eip155ExactClient::new(signer));

    // Build client with x402 middleware
    let client_builder = reqwest::Client::builder();
    let client_builder = if req_config.follow_redirects {
        client_builder.redirect(reqwest::redirect::Policy::limited(10))
    } else {
        client_builder.redirect(reqwest::redirect::Policy::none())
    };

    let client: ClientWithMiddleware = client_builder
        .build()?
        .with_payments(x402_client)
        .build();

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
        response.into(),
        args.output.as_deref(),
        args.fail,
        verbose,
    ).await?;

    Ok(())
}
