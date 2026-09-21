//! Optional counterparty check. Amount and `--confirm` still decide first.
//! The program runs only after those pins, and only before any signature.
//! Leave `--x402-before-sign` unset and this module is never called.

use crate::error::X402Error;
use reqwest::header::HeaderMap;
use reqwest::Method;
use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};
use x402_types::scheme::client::MaxAmount;
use x402_types::util::Base64Bytes;

/// The accept a clearance can bind to. `amount` is the decimal atomic string
/// from the 402, not a re-encoded value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClearanceTerms {
    pub url: String,
    pub pay_to: String,
    pub network: String,
    pub asset: String,
    pub scheme: String,
    pub amount: String,
    pub resource: String,
}

pub struct ApprovedPayment {
    pub pay_to: String,
    pub amount: MaxAmount,
    pub asset: String,
}

pub enum BeforeSignOutcome {
    /// `--confirm` declined. Nothing was signed.
    Cancelled,
    /// The probe was not a 402. A later 402 must not be signed either.
    NoPayment,
    /// The program exited 0 for this accept.
    Approved(ApprovedPayment),
}

pub fn prompt_confirmation(amount: &str, recipient: &str) -> Result<bool, X402Error> {
    eprint!(
        "Payment required: {}\nRecipient: {}\nProceed? [y/N] ",
        amount, recipient
    );
    std::io::stderr()
        .flush()
        .map_err(|e| X402Error::General(e.to_string()))?;

    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| X402Error::General(e.to_string()))?;

    Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
}

pub async fn evaluate_before_sign(
    method: Method,
    url: &str,
    headers: HeaderMap,
    program: &str,
    max_amount: Option<MaxAmount>,
    confirm: bool,
) -> Result<BeforeSignOutcome, X402Error> {
    let response = reqwest::Client::new()
        .request(method, url)
        .headers(headers)
        .send()
        .await?;

    if response.status() != reqwest::StatusCode::PAYMENT_REQUIRED {
        return Ok(BeforeSignOutcome::NoPayment);
    }

    let response_headers = response.headers().clone();
    let body = response.text().await.unwrap_or_default();
    let challenge = challenge_json(&response_headers, &body)
        .ok_or_else(|| X402Error::BeforeSign("No matching payment option".into()))?;

    let Some(terms) = select_terms(&challenge, url, max_amount) else {
        // Same refusal the amount selector uses. The program is not run.
        return Err(X402Error::BeforeSign("No matching payment option".into()));
    };

    if confirm && !prompt_confirmation(&terms.amount, &terms.pay_to)? {
        return Ok(BeforeSignOutcome::Cancelled);
    }

    run_program(program, &terms)?;
    let amount = MaxAmount(
        terms
            .amount
            .parse()
            .map_err(|_| X402Error::BeforeSign("No matching payment option".into()))?,
    );
    Ok(BeforeSignOutcome::Approved(ApprovedPayment {
        pay_to: terms.pay_to,
        amount,
        asset: terms.asset,
    }))
}

fn challenge_json(headers: &HeaderMap, body: &str) -> Option<Value> {
    if let Some(header) = headers
        .get("payment-required")
        .and_then(|v| v.to_str().ok())
    {
        if let Ok(decoded) = Base64Bytes::from(header.as_bytes()).decode() {
            if let Ok(value) = serde_json::from_slice::<Value>(&decoded) {
                return Some(value);
            }
        }
    }
    serde_json::from_str(body).ok()
}

/// First accept at or under the ceiling, in listed order. No ceiling keeps
/// the first accept. An accept with no parseable amount is skipped.
pub fn select_terms(
    challenge: &Value,
    request_url: &str,
    max_amount: Option<MaxAmount>,
) -> Option<ClearanceTerms> {
    let accepts = challenge.get("accepts")?.as_array()?;
    for accept in accepts {
        let amount = json_str(accept, &["maxAmountRequired", "amount"])?;
        let parsed = MaxAmount(amount.parse().ok()?);
        if max_amount.as_ref().is_some_and(|max| parsed.0 > max.0) {
            continue;
        }
        let pay_to = json_str(accept, &["payTo", "pay_to"])?;
        return Some(ClearanceTerms {
            url: request_url.to_string(),
            pay_to,
            network: json_str(accept, &["network"]).unwrap_or_default(),
            asset: json_str(accept, &["asset"]).unwrap_or_default(),
            scheme: json_str(accept, &["scheme"]).unwrap_or_else(|| "exact".into()),
            amount,
            resource: resource_of(challenge, accept, request_url),
        });
    }
    None
}

fn resource_of(challenge: &Value, accept: &Value, request_url: &str) -> String {
    match challenge.get("resource") {
        Some(Value::String(url)) if !url.is_empty() => return url.clone(),
        Some(Value::Object(obj)) => {
            if let Some(url) = obj.get("url").and_then(Value::as_str) {
                if !url.is_empty() {
                    return url.to_string();
                }
            }
        }
        _ => {}
    }
    json_str(accept, &["resource"]).unwrap_or_else(|| request_url.to_string())
}

fn json_str(value: &Value, keys: &[&str]) -> Option<String> {
    for key in keys {
        if let Some(text) = value.get(*key).and_then(Value::as_str) {
            if !text.is_empty() {
                return Some(text.to_string());
            }
        }
    }
    None
}

fn run_program(program: &str, terms: &ClearanceTerms) -> Result<(), X402Error> {
    let payload = serde_json::to_vec(&serde_json::json!({
        "url": terms.url,
        "payTo": terms.pay_to,
        "network": terms.network,
        "asset": terms.asset,
        "scheme": terms.scheme,
        "amount": terms.amount,
        "resource": terms.resource,
    }))
    .map_err(|e| X402Error::General(e.to_string()))?;

    let mut child = Command::new(program)
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| X402Error::General(format!("Invalid --x402-before-sign '{program}': {e}")))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(&payload)
            .map_err(|e| X402Error::General(e.to_string()))?;
    }

    let output = child
        .wait_with_output()
        .map_err(|e| X402Error::General(e.to_string()))?;
    if output.status.success() {
        return Ok(());
    }

    let detail = String::from_utf8_lossy(&output.stderr);
    let detail = detail.trim();
    let detail: String = if detail.is_empty() {
        "before-sign refused the payment. Nothing was signed.".into()
    } else {
        detail.chars().take(500).collect()
    };
    Err(X402Error::BeforeSign(format!(
        "before-sign refused: {detail}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v2() -> Value {
        serde_json::json!({
            "x402Version": 2,
            "resource": { "url": "https://seller.example/paid" },
            "accepts": [
                {
                    "scheme": "exact",
                    "network": "eip155:8453",
                    "amount": "10001",
                    "payTo": "0x1111111111111111111111111111111111111111",
                    "asset": "0xusdc"
                },
                {
                    "scheme": "exact",
                    "network": "eip155:8453",
                    "amount": "1000",
                    "payTo": "0x2222222222222222222222222222222222222222",
                    "asset": "0xusdc"
                }
            ]
        })
    }

    #[test]
    fn over_ceiling_is_skipped_and_the_next_accept_is_kept() {
        let terms = select_terms(
            &v2(),
            "https://request.example",
            Some(MaxAmount("10000".parse().unwrap())),
        )
        .unwrap();
        assert_eq!(terms.pay_to, "0x2222222222222222222222222222222222222222");
        assert_eq!(terms.amount, "1000");
        assert_eq!(terms.resource, "https://seller.example/paid");
        assert_eq!(terms.network, "eip155:8453");
    }

    #[test]
    fn nothing_under_the_ceiling_selects_nothing() {
        let terms = select_terms(
            &v2(),
            "https://request.example",
            Some(MaxAmount("999".parse().unwrap())),
        );
        assert!(terms.is_none());
    }

    #[test]
    fn no_ceiling_keeps_the_first_accept() {
        let terms = select_terms(&v2(), "https://request.example", None).unwrap();
        assert_eq!(terms.pay_to, "0x1111111111111111111111111111111111111111");
        assert_eq!(terms.amount, "10001");
    }
}
