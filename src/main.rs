mod balance;
mod before_sign;
mod cli;
mod config;
mod error;
mod output;
mod request;

use before_sign::{evaluate_before_sign, ApprovedPayment, BeforeSignOutcome};
use cli::Args;
use config::Config;
use error::X402Error;
use output::handle_response;
use request::RequestConfig;
use reqwest_middleware::ClientWithMiddleware;
use std::process::ExitCode;
use std::sync::Arc;
use x402_chain_eip155::v1_eip155_exact::client::V1Eip155ExactClient;
use x402_chain_eip155::v2_eip155_exact::client::V2Eip155ExactClient;
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};
use x402_types::scheme::client::{FirstMatch, MaxAmount, PaymentCandidate, PaymentSelector};

/// Keeps the amount pin, then requires the accept the before-sign program saw.
struct BeforeSignGate<S> {
    inner: S,
    approved: ApprovedPayment,
}

impl<S: PaymentSelector> PaymentSelector for BeforeSignGate<S> {
    fn select<'a>(&self, candidates: &'a [PaymentCandidate]) -> Option<&'a PaymentCandidate> {
        let chosen = self.inner.select(candidates)?;
        let same_payee = chosen.pay_to.eq_ignore_ascii_case(&self.approved.pay_to);
        let same_amount = chosen.amount == self.approved.amount.0;
        let same_asset = self.approved.asset.is_empty()
            || chosen.asset.eq_ignore_ascii_case(&self.approved.asset);
        if same_payee && same_amount && same_asset {
            Some(chosen)
        } else {
            None
        }
    }
}

struct RefuseAll;

impl PaymentSelector for RefuseAll {
    fn select<'a>(&self, _: &'a [PaymentCandidate]) -> Option<&'a PaymentCandidate> {
        None
    }
}

fn parse_payment_info(body: &str) -> (String, String) {
    // Try to parse JSON payment info
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        let amount = json
            .get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown amount");
        let recipient = json
            .get("recipient")
            .or_else(|| json.get("payTo"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        (amount.to_string(), recipient.to_string())
    } else {
        ("unknown amount".to_string(), "unknown".to_string())
    }
}

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
    let max_amount = match args.x402_max_amount.as_deref() {
        Some(value) if !value.is_empty() && value.bytes().all(|byte| byte.is_ascii_digit()) => {
            Some(MaxAmount(value.parse().map_err(|_| {
                X402Error::General(format!(
                    "Invalid --x402-max-amount '{value}': expected non-negative decimal atomic units"
                ))
            })?))
        }
        Some(value) => {
            return Err(X402Error::General(format!(
                "Invalid --x402-max-amount '{value}': expected non-negative decimal atomic units"
            )))
        }
        None => None,
    };
    let config = Config::load(
        args.x402_key.as_deref(),
        args.x402_wallet.as_deref(),
        args.x402_wallet_password.as_deref(),
    )?;

    if args.x402_balance {
        let signer = config.require_signer()?;
        let rpc_url = balance::resolve_rpc(args.x402_rpc_url.as_deref());
        return balance::query_balance(&signer, &rpc_url, args.x402_token.as_deref()).await;
    }

    let req_config = RequestConfig::from_args(&args)?;

    let verbose = args.verbose || config.verbose;

    if verbose {
        eprintln!("> {} {}", req_config.method, req_config.url);
        for (name, value) in req_config.headers.iter() {
            eprintln!("> {}: {}", name, value.to_str().unwrap_or("<binary>"));
        }
    }

    // Dry-run mode: make request without payment handling
    if args.x402_dry_run {
        return dry_run(&req_config, verbose).await;
    }

    let want_confirm = args.confirm || config.confirm;
    let gate = if let Some(program) = args.x402_before_sign.as_deref() {
        if program.is_empty() {
            return Err(X402Error::General(
                "Invalid --x402-before-sign: expected a program".into(),
            ));
        }
        Some(
            evaluate_before_sign(
                req_config.method.clone(),
                &req_config.url,
                req_config.headers.clone(),
                program,
                max_amount.as_ref().map(|max| MaxAmount(max.0)),
                want_confirm,
            )
            .await?,
        )
    } else {
        None
    };

    if matches!(gate, Some(BeforeSignOutcome::Cancelled)) {
        eprintln!("Payment cancelled.");
        return Ok(());
    }

    // Confirmation mode, unchanged when before-sign is unset.
    if gate.is_none() && want_confirm {
        let preflight_client = reqwest::Client::new();
        let preflight_response = preflight_client
            .request(req_config.method.clone(), &req_config.url)
            .headers(req_config.headers.clone())
            .send()
            .await?;

        if preflight_response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
            let body = preflight_response.text().await.unwrap_or_default();
            let (amount, recipient) = parse_payment_info(&body);

            if !before_sign::prompt_confirmation(&amount, &recipient)? {
                eprintln!("Payment cancelled.");
                return Ok(());
            }
        }
    }

    // Get signer from private key or wallet keystore
    let signer = config.require_signer()?;

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

    let client: ClientWithMiddleware = match gate {
        Some(BeforeSignOutcome::NoPayment) => client_builder
            .build()?
            .with_payments(x402_client.with_selector(RefuseAll))
            .build(),
        Some(BeforeSignOutcome::Approved(approved)) => {
            if let Some(max_amount) = max_amount {
                client_builder
                    .build()?
                    .with_payments(x402_client.with_selector(BeforeSignGate {
                        inner: max_amount,
                        approved,
                    }))
                    .build()
            } else {
                client_builder
                    .build()?
                    .with_payments(x402_client.with_selector(BeforeSignGate {
                        inner: FirstMatch,
                        approved,
                    }))
                    .build()
            }
        }
        Some(BeforeSignOutcome::Cancelled) => unreachable!("cancel returns before the client"),
        None => {
            if let Some(max_amount) = max_amount {
                client_builder
                    .build()?
                    .with_payments(x402_client.with_selector(max_amount))
                    .build()
            } else {
                client_builder.build()?.with_payments(x402_client).build()
            }
        }
    };

    // Build request
    let mut request = client.request(req_config.method, &req_config.url);
    request = request.headers(req_config.headers);

    // Form data takes precedence over body
    if let Some(form) = RequestConfig::parse_form(&args.form)? {
        request = request.multipart(form);
    } else if let Some(body) = req_config.body {
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
    handle_response(response, args.output.as_deref(), args.fail, verbose).await?;

    Ok(())
}

async fn dry_run(req_config: &RequestConfig, verbose: bool) -> Result<(), X402Error> {
    let client = reqwest::Client::new();
    let response = client
        .request(req_config.method.clone(), &req_config.url)
        .headers(req_config.headers.clone())
        .send()
        .await?;

    if response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        eprintln!("Payment required:");

        // Try to extract x402 headers
        for (name, value) in response.headers() {
            let name_str = name.as_str().to_lowercase();
            if name_str.starts_with("x-402") || name_str.starts_with("x402") {
                eprintln!("  {}: {}", name, value.to_str().unwrap_or("<binary>"));
            }
        }

        // Try to parse body for payment details
        let body = response.text().await.unwrap_or_default();
        if !body.is_empty() {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                eprintln!(
                    "  Payment details: {}",
                    serde_json::to_string_pretty(&json).unwrap_or(body)
                );
            } else {
                eprintln!("  Body: {}", body);
            }
        }

        eprintln!("(dry run - no payment made)");
    } else if verbose {
        eprintln!("< {} (no payment required)", response.status());
    }

    Ok(())
}
