use assert_cmd::Command;
use predicates::prelude::*;
use std::io::Write;
use tempfile::NamedTempFile;

/// Keystore v3 JSON for Hardhat #0 key (0xac0974bec...f2ff80) with password "testpassword123"
const TEST_KEYSTORE_JSON: &str = r#"{"address":"f39Fd6e51aad88F6F4ce6aB8827279cffFb92266","crypto":{"cipher":"aes-128-ctr","cipherparams":{"iv":"27f2444b8bfd4b13eeb89843ca857e6e"},"ciphertext":"121498fa631ea0bbaf027808fa79115692860ecae5db2a73d4cf7dd56209d045","kdf":"scrypt","kdfparams":{"dklen":32,"n":262144,"r":8,"p":1,"salt":"c6113dae558dcd445bb99c3a47c6198e"},"mac":"c29d0c74e3462191091e7ae5fcbf2b219492d9cb6e469b6abbdd540eab72710e"},"id":"6b480f64-c657-4924-928d-00256e3e6a1d","version":3}"#;

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
        .stdout(predicate::str::contains("--x402-wallet"))
        .stdout(predicate::str::contains("--x402-balance"))
        .stdout(predicate::str::contains("--x402-rpc-url"))
        .stdout(predicate::str::contains("--x402-token"));
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
