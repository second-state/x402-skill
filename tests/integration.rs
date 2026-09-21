use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use std::os::unix::fs::PermissionsExt;
use tempfile::NamedTempFile;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};
use x402_types::util::Base64Bytes;

/// Keystore v3 JSON for Hardhat #0 key (0xac0974bec...f2ff80) with password "testpassword123"
const TEST_KEYSTORE_JSON: &str = r#"{"address":"f39Fd6e51aad88F6F4ce6aB8827279cffFb92266","crypto":{"cipher":"aes-128-ctr","cipherparams":{"iv":"27f2444b8bfd4b13eeb89843ca857e6e"},"ciphertext":"121498fa631ea0bbaf027808fa79115692860ecae5db2a73d4cf7dd56209d045","kdf":"scrypt","kdfparams":{"dklen":32,"n":262144,"r":8,"p":1,"salt":"c6113dae558dcd445bb99c3a47c6198e"},"mac":"c29d0c74e3462191091e7ae5fcbf2b219492d9cb6e469b6abbdd540eab72710e"},"id":"6b480f64-c657-4924-928d-00256e3e6a1d","version":3}"#;
const TEST_PRIVATE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
const TEST_PAY_TO: &str = "0x1111111111111111111111111111111111111111";
const TEST_SECOND_PAY_TO: &str = "0x2222222222222222222222222222222222222222";
const TEST_ASSET: &str = "0x036CbD53842c5426634e7929541eC2318f3dCF7e";

fn v1_challenge(server: &MockServer, amount: &str) -> serde_json::Value {
    serde_json::json!({
        "x402Version": 1,
        "accepts": [{
            "scheme": "exact",
            "network": "base-sepolia",
            "maxAmountRequired": amount,
            "resource": format!("{}/requested-resource", server.uri()),
            "description": "local max-amount fixture",
            "mimeType": "application/json",
            "payTo": TEST_PAY_TO,
            "maxTimeoutSeconds": 300,
            "asset": TEST_ASSET,
            "extra": {"name": "USD Coin", "version": "2"}
        }]
    })
}

fn v2_challenge(server: &MockServer, amount: &str) -> serde_json::Value {
    serde_json::json!({
        "x402Version": 2,
        "resource": {
            "url": format!("{}/requested-resource", server.uri()),
            "description": "local max-amount fixture",
            "mimeType": "application/json"
        },
        "accepts": [{
            "scheme": "exact",
            "network": "eip155:84532",
            "amount": amount,
            "payTo": TEST_PAY_TO,
            "maxTimeoutSeconds": 300,
            "asset": TEST_ASSET,
            "extra": {
                "assetTransferMethod": "eip3009",
                "name": "USD Coin",
                "version": "2"
            }
        }]
    })
}

async fn mount_v1_challenge(server: &MockServer, amount: &str) {
    let challenge = v1_challenge(server, amount);
    Mock::given(method("GET"))
        .and(path("/requested-resource"))
        .respond_with(move |request: &Request| {
            if request.headers.contains_key("x-payment") {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true}))
            } else {
                ResponseTemplate::new(402).set_body_json(challenge.clone())
            }
        })
        .mount(server)
        .await;
}

async fn mount_v2_payment_required(server: &MockServer, challenge: serde_json::Value) {
    let encoded = Base64Bytes::encode(challenge.to_string()).to_string();
    Mock::given(method("GET"))
        .and(path("/requested-resource"))
        .respond_with(move |request: &Request| {
            if request.headers.contains_key("payment-signature") {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({"ok": true}))
            } else {
                ResponseTemplate::new(402).insert_header("Payment-Required", encoded.as_str())
            }
        })
        .mount(server)
        .await;
}

async fn mount_v2_challenge(server: &MockServer, amount: &str) {
    mount_v2_payment_required(server, v2_challenge(server, amount)).await;
}

fn payment_command(server: &MockServer, max_amount: Option<&str>) -> Command {
    let mut command = Command::cargo_bin("x402curl").unwrap();
    if let Some(max_amount) = max_amount {
        command.args(["--x402-max-amount", max_amount]);
    }
    command
        .arg(format!("{}/requested-resource", server.uri()))
        .env("X402_PRIVATE_KEY", TEST_PRIVATE_KEY)
        .env_remove("X402_WALLET")
        .env_remove("X402_WALLET_PASSWORD");
    command
}

fn assert_unpaid_request(request: &Request) {
    assert!(!request.headers.contains_key("x-payment"));
    assert!(!request.headers.contains_key("payment-signature"));
    assert_eq!(request.url.path(), "/requested-resource");
}

fn decoded_payment_header(request: &Request, name: &str) -> serde_json::Value {
    let encoded = request.headers.get(name).unwrap().to_str().unwrap();
    let decoded = Base64Bytes::from(encoded.as_bytes()).decode().unwrap();
    serde_json::from_slice(&decoded).unwrap()
}

fn write_test_keystore() -> NamedTempFile {
    let mut file = NamedTempFile::new().unwrap();
    file.write_all(TEST_KEYSTORE_JSON.as_bytes()).unwrap();
    file
}

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("x402curl"))
        .stdout(predicate::str::contains("--x402-dry-run"))
        .stdout(predicate::str::contains("--x402-max-amount"))
        .stdout(predicate::str::contains("--x402-before-sign"))
        .stdout(predicate::str::contains("--x402-wallet"))
        .stdout(predicate::str::contains("--x402-balance"))
        .stdout(predicate::str::contains("--x402-rpc-url"))
        .stdout(predicate::str::contains("--x402-token"));
}

#[test]
fn test_max_amount_rejects_invalid_values_before_credentials_or_network() {
    for value in [
        "-1",
        "not-a-number",
        "0x10",
        "1_000",
        "999999999999999999999999999999999999999999999999999999999999999999999999999999",
    ] {
        let mut command = Command::cargo_bin("x402curl").unwrap();
        command
            .args(["--x402-max-amount", value, "http://127.0.0.1:9/unreachable"])
            .env_remove("X402_PRIVATE_KEY")
            .env_remove("X402_WALLET")
            .env_remove("X402_WALLET_PASSWORD")
            .assert()
            .failure()
            .code(1)
            .stderr(predicate::str::contains("Invalid --x402-max-amount"));
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_amount_rejects_oversized_v1_before_signing() {
    let server = MockServer::start().await;
    mount_v1_challenge(&server, "10001").await;

    payment_command(&server, Some("10000"))
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No matching payment option"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_unpaid_request(&requests[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_amount_rejects_oversized_v2_before_signing() {
    let server = MockServer::start().await;
    mount_v2_challenge(&server, "10001").await;

    payment_command(&server, Some("10000"))
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No matching payment option"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_unpaid_request(&requests[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_amount_accepts_equal_v1_and_v2_quotes() {
    let v1_server = MockServer::start().await;
    mount_v1_challenge(&v1_server, "10000").await;
    payment_command(&v1_server, Some("10000"))
        .assert()
        .success();
    let v1_requests = v1_server.received_requests().await.unwrap();
    assert_eq!(v1_requests.len(), 2);
    assert_unpaid_request(&v1_requests[0]);
    assert!(v1_requests[1].headers.contains_key("x-payment"));
    assert!(!v1_requests[1].headers.contains_key("payment-signature"));
    assert_eq!(v1_requests[1].url.path(), "/requested-resource");

    let v2_server = MockServer::start().await;
    mount_v2_challenge(&v2_server, "10000").await;
    payment_command(&v2_server, Some("10000"))
        .assert()
        .success();
    let v2_requests = v2_server.received_requests().await.unwrap();
    assert_eq!(v2_requests.len(), 2);
    assert_unpaid_request(&v2_requests[0]);
    assert!(v2_requests[1].headers.contains_key("payment-signature"));
    assert!(!v2_requests[1].headers.contains_key("x-payment"));
    assert_eq!(v2_requests[1].url.path(), "/requested-resource");
}

#[tokio::test(flavor = "multi_thread")]
async fn test_max_amount_selects_later_in_cap_v2_offer() {
    let server = MockServer::start().await;
    let mut challenge = v2_challenge(&server, "10001");
    let mut in_cap_offer = challenge["accepts"][0].clone();
    in_cap_offer["amount"] = serde_json::json!("9999");
    in_cap_offer["payTo"] = serde_json::json!(TEST_SECOND_PAY_TO);
    challenge["accepts"]
        .as_array_mut()
        .unwrap()
        .push(in_cap_offer);
    mount_v2_payment_required(&server, challenge).await;

    payment_command(&server, Some("10000")).assert().success();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_unpaid_request(&requests[0]);
    let payment = decoded_payment_header(&requests[1], "payment-signature");
    assert_eq!(payment.pointer("/accepted/amount").unwrap(), "9999");
    assert_eq!(
        payment.pointer("/accepted/payTo").unwrap(),
        TEST_SECOND_PAY_TO
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn test_no_max_amount_preserves_existing_first_match_behavior() {
    let server = MockServer::start().await;
    mount_v2_challenge(&server, "999999999999999999999999").await;

    payment_command(&server, None).assert().success();

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 2);
    assert_unpaid_request(&requests[0]);
    assert!(requests[1].headers.contains_key("payment-signature"));
}

#[tokio::test(flavor = "multi_thread")]
async fn test_before_sign_refuses_before_signing_without_a_key() {
    let server = MockServer::start().await;
    mount_v2_challenge(&server, "1000").await;

    Command::cargo_bin("x402curl")
        .unwrap()
        .args([
            "--x402-before-sign",
            "/bin/false",
            &format!("{}/requested-resource", server.uri()),
        ])
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .env_remove("X402_WALLET_PASSWORD")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("before-sign refused"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_unpaid_request(&requests[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_before_sign_is_not_called_when_the_amount_pin_misses() {
    let server = MockServer::start().await;
    mount_v2_challenge(&server, "10001").await;
    let dir = tempfile::tempdir().unwrap();
    let marker = dir.path().join("called");
    let program = dir.path().join("hook.sh");
    std::fs::write(
        &program,
        format!("#!/bin/sh\ntouch '{}'\nexit 1\n", marker.display()),
    )
    .unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();

    Command::cargo_bin("x402curl")
        .unwrap()
        .args([
            "--x402-max-amount",
            "10000",
            "--x402-before-sign",
            program.to_str().unwrap(),
            &format!("{}/requested-resource", server.uri()),
        ])
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .env_remove("X402_WALLET_PASSWORD")
        .assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("No matching payment option"));

    assert!(!marker.exists());
    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_unpaid_request(&requests[0]);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_before_sign_allow_still_signs_the_same_accept() {
    let server = MockServer::start().await;
    mount_v2_challenge(&server, "1000").await;

    Command::cargo_bin("x402curl")
        .unwrap()
        .args([
            "--x402-before-sign",
            "/bin/true",
            &format!("{}/requested-resource", server.uri()),
        ])
        .env("X402_PRIVATE_KEY", TEST_PRIVATE_KEY)
        .env_remove("X402_WALLET")
        .env_remove("X402_WALLET_PASSWORD")
        .assert()
        .success();

    let requests = server.received_requests().await.unwrap();
    assert!(requests.len() >= 2);
    assert_unpaid_request(&requests[0]);
    assert!(requests
        .last()
        .unwrap()
        .headers
        .contains_key("payment-signature"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--version").assert().success();
}

#[test]
fn test_missing_url() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_missing_key_error() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://example.com")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .stderr(predicate::str::contains("No wallet credentials found"));
}

#[test]
fn test_dry_run_no_key_required() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-dry-run")
        .arg("https://httpbin.org/status/200")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .success();
}

#[test]
fn test_basic_get_request() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .env(
            "X402_PRIVATE_KEY",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("httpbin.org"));
}

#[test]
fn test_post_with_data() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args([
        "-X",
        "POST",
        "-d",
        "{\"test\":1}",
        "https://httpbin.org/post",
    ])
    .env(
        "X402_PRIVATE_KEY",
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    )
    .assert()
    .success()
    .stdout(predicate::str::contains("test"));
}

#[test]
fn test_custom_header() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args(["-H", "X-Custom: value", "https://httpbin.org/headers"])
        .env(
            "X402_PRIVATE_KEY",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .assert()
        .success()
        .stdout(predicate::str::contains("X-Custom"));
}

#[test]
fn test_fail_on_404() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args(["-f", "https://httpbin.org/status/404"])
        .env(
            "X402_PRIVATE_KEY",
            "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
        )
        .assert()
        .failure()
        .code(4);
}

// Keystore wallet tests

#[test]
fn test_wallet_keystore_loads() {
    let keystore_file = write_test_keystore();
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .arg("--x402-wallet")
        .arg(keystore_file.path())
        .arg("--x402-wallet-password")
        .arg("testpassword123")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .success()
        .stdout(predicate::str::contains("httpbin.org"));
}

#[test]
fn test_wallet_missing_password() {
    let keystore_file = write_test_keystore();
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .arg("--x402-wallet")
        .arg(keystore_file.path())
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("no password provided"));
}

#[test]
fn test_wallet_wrong_password() {
    let keystore_file = write_test_keystore();
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .arg("--x402-wallet")
        .arg(keystore_file.path())
        .arg("--x402-wallet-password")
        .arg("wrongpassword")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("Failed to decrypt keystore"));
}

#[test]
fn test_wallet_file_not_found() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .arg("--x402-wallet")
        .arg("/nonexistent/wallet.json")
        .arg("--x402-wallet-password")
        .arg("password")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn test_private_key_takes_priority_over_wallet() {
    // Private key should be used even when a (nonexistent) wallet is specified
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .arg("--x402-key")
        .arg("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .arg("--x402-wallet")
        .arg("/nonexistent/wallet.json")
        .arg("--x402-wallet-password")
        .arg("password")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .success();
}

// Balance command tests

#[test]
fn test_balance_no_url_required() {
    // --x402-balance should work without providing a URL
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-balance")
        .arg("--x402-key")
        .arg("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .success()
        .stderr(predicate::str::contains("USDC"))
        .stderr(predicate::str::contains("Address:"));
}

#[test]
fn test_balance_no_credentials() {
    // --x402-balance without credentials should fail with exit code 5
    // Use a temp dir as CWD so dotenvy::dotenv() won't find the project .env file
    let tmp = tempfile::tempdir().unwrap();
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-balance")
        .current_dir(tmp.path())
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("No wallet credentials found"));
}

#[test]
fn test_url_still_required_without_balance() {
    // Without --x402-balance, URL should still be required
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-key")
        .arg("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_balance_custom_token() {
    // --x402-token with Base mainnet USDC address should work and auto-detect symbol
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-balance")
        .arg("--x402-token")
        .arg("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")
        .arg("--x402-key")
        .arg("0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .env_remove("X402_PRIVATE_KEY")
        .env_remove("X402_WALLET")
        .assert()
        .success()
        .stderr(predicate::str::contains("Address:"))
        .stderr(predicate::str::contains("Network:"));
}
