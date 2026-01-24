use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("x402curl"))
        .stdout(predicate::str::contains("--x402-dry-run"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--version")
        .assert()
        .success();
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
        .assert()
        .failure()
        .stderr(predicate::str::contains("No private key found"));
}

#[test]
fn test_dry_run_no_key_required() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("--x402-dry-run")
        .arg("https://httpbin.org/status/200")
        .env_remove("X402_PRIVATE_KEY")
        .assert()
        .success();
}

#[test]
fn test_basic_get_request() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.arg("https://httpbin.org/get")
        .env("X402_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .assert()
        .success()
        .stdout(predicate::str::contains("httpbin.org"));
}

#[test]
fn test_post_with_data() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args(["-X", "POST", "-d", "{\"test\":1}", "https://httpbin.org/post"])
        .env("X402_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .assert()
        .success()
        .stdout(predicate::str::contains("test"));
}

#[test]
fn test_custom_header() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args(["-H", "X-Custom: value", "https://httpbin.org/headers"])
        .env("X402_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .assert()
        .success()
        .stdout(predicate::str::contains("X-Custom"));
}

#[test]
fn test_fail_on_404() {
    let mut cmd = Command::cargo_bin("x402curl").unwrap();
    cmd.args(["-f", "https://httpbin.org/status/404"])
        .env("X402_PRIVATE_KEY", "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80")
        .assert()
        .failure()
        .code(4);
}
