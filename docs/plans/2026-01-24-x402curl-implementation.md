# x402curl Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build a curl-like CLI tool that automatically handles x402 payments using x402-reqwest.

**Architecture:** Thin CLI wrapper around x402-reqwest. Parse curl-compatible flags with clap, load private key from config hierarchy, build reqwest request, let x402-reqwest handle 402 responses transparently.

**Tech Stack:** Rust, clap (CLI), x402-reqwest, tokio (async), dotenvy (.env), toml (config), thiserror (errors)

---

## Task 1: Project Skeleton

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/error.rs`

**Step 1: Create Cargo.toml**

```toml
[package]
name = "x402curl"
version = "0.1.0"
edition = "2021"
description = "curl with automatic x402 payment handling"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
x402-rs = { git = "https://github.com/x402-rs/x402-rs" }
x402-reqwest = { git = "https://github.com/x402-rs/x402-rs" }
reqwest = { version = "0.12", features = ["multipart", "json"] }
reqwest-middleware = "0.4"
dotenvy = "0.15"
toml = "0.8"
dirs = "6"
thiserror = "2"
alloy-signer-local = "1"

[dev-dependencies]
wiremock = "0.6"
assert_cmd = "2"
predicates = "3"
tempfile = "3"
```

**Step 2: Create src/error.rs**

```rust
use std::process::ExitCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum X402Error {
    #[error("General error: {0}")]
    General(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Payment error: {0}")]
    Payment(String),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Configuration error: {0}")]
    Config(String),
}

impl X402Error {
    pub fn exit_code(&self) -> ExitCode {
        match self {
            X402Error::General(_) => ExitCode::from(1),
            X402Error::Network(_) => ExitCode::from(2),
            X402Error::Payment(_) => ExitCode::from(3),
            X402Error::Http(_) => ExitCode::from(4),
            X402Error::Config(_) => ExitCode::from(5),
        }
    }

    pub fn code_number(&self) -> u8 {
        match self {
            X402Error::General(_) => 1,
            X402Error::Network(_) => 2,
            X402Error::Payment(_) => 3,
            X402Error::Http(_) => 4,
            X402Error::Config(_) => 5,
        }
    }
}
```

**Step 3: Create src/main.rs**

```rust
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
```

**Step 4: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully (may take time to download dependencies)

**Step 5: Commit**

```bash
git add Cargo.toml src/
git commit -m "feat: initialize x402curl project skeleton"
```

---

## Task 2: CLI Argument Parsing

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`

**Step 1: Create src/cli.rs**

```rust
use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "x402curl")]
#[command(about = "curl with automatic x402 payment handling")]
#[command(version)]
pub struct Args {
    /// URL to request
    #[arg(required = true)]
    pub url: String,

    /// HTTP method (GET, POST, PUT, DELETE, etc.)
    #[arg(short = 'X', long = "request", default_value = "GET")]
    pub method: String,

    /// Add header (can be used multiple times)
    #[arg(short = 'H', long = "header", action = clap::ArgAction::Append)]
    pub headers: Vec<String>,

    /// Request body data (use @filename to read from file)
    #[arg(short = 'd', long = "data")]
    pub data: Option<String>,

    /// Send data without processing (use @filename to read from file)
    #[arg(long = "data-binary")]
    pub data_binary: Option<String>,

    /// Write output to file instead of stdout
    #[arg(short = 'o', long = "output")]
    pub output: Option<String>,

    /// Fail silently on HTTP errors (exit non-zero)
    #[arg(short = 'f', long = "fail")]
    pub fail: bool,

    /// Silent mode (suppress progress output)
    #[arg(short = 's', long = "silent")]
    pub silent: bool,

    /// Verbose mode (show payment details)
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Multipart form field (can be used multiple times)
    #[arg(short = 'F', long = "form", action = clap::ArgAction::Append)]
    pub form: Vec<String>,

    /// Basic auth (user:password)
    #[arg(short = 'u', long = "user")]
    pub user: Option<String>,

    /// Follow redirects
    #[arg(short = 'L', long = "location")]
    pub location: bool,

    /// Override private key
    #[arg(long = "x402-key")]
    pub x402_key: Option<String>,

    /// Show payment requirements without paying
    #[arg(long = "x402-dry-run")]
    pub x402_dry_run: bool,

    /// Prompt before making payment
    #[arg(long = "confirm")]
    pub confirm: bool,
}

impl Args {
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
```

**Step 2: Update src/main.rs**

```rust
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
```

**Step 3: Verify it works**

Run: `cargo run -- -X POST -H "Content-Type: application/json" https://example.com`
Expected: Prints URL and Method

Run: `cargo run -- --help`
Expected: Shows help with all flags

**Step 4: Commit**

```bash
git add src/cli.rs src/main.rs
git commit -m "feat: add CLI argument parsing with clap"
```

---

## Task 3: Configuration Loading

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

**Step 1: Create src/config.rs**

```rust
use crate::error::X402Error;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Default)]
pub struct Config {
    pub private_key: Option<String>,
    pub verbose: bool,
    pub confirm: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ConfigFile {
    private_key: Option<String>,
    #[serde(default)]
    defaults: ConfigDefaults,
}

#[derive(Debug, Default, serde::Deserialize)]
struct ConfigDefaults {
    #[serde(default)]
    verbose: bool,
    #[serde(default)]
    confirm: bool,
}

impl Config {
    /// Load configuration from hierarchy:
    /// 1. CLI flag (x402_key)
    /// 2. X402_PRIVATE_KEY env var
    /// 3. ./.env file
    /// 4. ~/.x402/config
    pub fn load(cli_key: Option<&str>) -> Result<Self, X402Error> {
        // Priority 1: CLI flag
        if let Some(key) = cli_key {
            return Ok(Config {
                private_key: Some(key.to_string()),
                ..Default::default()
            });
        }

        // Priority 2: Environment variable
        if let Ok(key) = std::env::var("X402_PRIVATE_KEY") {
            if !key.is_empty() {
                return Ok(Config {
                    private_key: Some(key),
                    ..Default::default()
                });
            }
        }

        // Priority 3: .env file in current directory
        if let Ok(_) = dotenvy::dotenv() {
            if let Ok(key) = std::env::var("X402_PRIVATE_KEY") {
                if !key.is_empty() {
                    return Ok(Config {
                        private_key: Some(key),
                        ..Default::default()
                    });
                }
            }
        }

        // Priority 4: ~/.x402/config
        if let Some(config_path) = Self::global_config_path() {
            if config_path.exists() {
                return Self::load_from_file(&config_path);
            }
        }

        // No key found
        Ok(Config::default())
    }

    fn global_config_path() -> Option<PathBuf> {
        dirs::home_dir().map(|h| h.join(".x402").join("config"))
    }

    fn load_from_file(path: &PathBuf) -> Result<Self, X402Error> {
        let content = fs::read_to_string(path)
            .map_err(|e| X402Error::Config(format!("Failed to read config file: {}", e)))?;

        let config_file: ConfigFile = toml::from_str(&content)
            .map_err(|e| X402Error::Config(format!("Failed to parse config file: {}", e)))?;

        Ok(Config {
            private_key: config_file.private_key,
            verbose: config_file.defaults.verbose,
            confirm: config_file.defaults.confirm,
        })
    }

    pub fn require_private_key(&self) -> Result<&str, X402Error> {
        self.private_key.as_deref().ok_or_else(|| {
            X402Error::Config(
                "No private key found. Set X402_PRIVATE_KEY or create ~/.x402/config".to_string(),
            )
        })
    }
}
```

**Step 2: Add serde to Cargo.toml dependencies**

Add this line to `[dependencies]` in Cargo.toml:
```toml
serde = { version = "1", features = ["derive"] }
```

**Step 3: Update src/main.rs**

```rust
mod cli;
mod config;
mod error;

use cli::Args;
use config::Config;
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
    let config = Config::load(args.x402_key.as_deref())?;

    let verbose = args.verbose || config.verbose;

    if verbose {
        eprintln!("> {} {}", args.method, args.url);
    }

    // For now, just verify we can load the key
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
```

**Step 4: Verify it works**

Run: `X402_PRIVATE_KEY=0x1234567890abcdef cargo run -- -v https://example.com`
Expected: Shows masked key

Run: `cargo run -- https://example.com`
Expected: Error [5]: No private key found...

**Step 5: Commit**

```bash
git add Cargo.toml src/config.rs src/main.rs
git commit -m "feat: add configuration loading with hierarchy"
```

---

## Task 4: Request Building

**Files:**
- Create: `src/request.rs`
- Modify: `src/main.rs`

**Step 1: Create src/request.rs**

```rust
use crate::cli::Args;
use crate::error::X402Error;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use reqwest::{Body, Method};
use std::fs;
use std::str::FromStr;

pub struct RequestConfig {
    pub method: Method,
    pub url: String,
    pub headers: HeaderMap,
    pub body: Option<Body>,
    pub follow_redirects: bool,
}

impl RequestConfig {
    pub fn from_args(args: &Args) -> Result<Self, X402Error> {
        let method = Method::from_str(&args.method.to_uppercase())
            .map_err(|_| X402Error::General(format!("Invalid HTTP method: {}", args.method)))?;

        let headers = Self::parse_headers(&args.headers)?;
        let body = Self::parse_body(&args.data, &args.data_binary)?;

        Ok(RequestConfig {
            method,
            url: args.url.clone(),
            headers,
            body,
            follow_redirects: args.location,
        })
    }

    fn parse_headers(headers: &[String]) -> Result<HeaderMap, X402Error> {
        let mut map = HeaderMap::new();
        for header in headers {
            let parts: Vec<&str> = header.splitn(2, ':').collect();
            if parts.len() != 2 {
                return Err(X402Error::General(format!("Invalid header format: {}", header)));
            }
            let name = HeaderName::from_str(parts[0].trim())
                .map_err(|_| X402Error::General(format!("Invalid header name: {}", parts[0])))?;
            let value = HeaderValue::from_str(parts[1].trim())
                .map_err(|_| X402Error::General(format!("Invalid header value: {}", parts[1])))?;
            map.insert(name, value);
        }
        Ok(map)
    }

    fn parse_body(data: &Option<String>, data_binary: &Option<String>) -> Result<Option<Body>, X402Error> {
        // data_binary takes precedence
        let data_str = data_binary.as_ref().or(data.as_ref());

        match data_str {
            Some(d) if d.starts_with('@') => {
                let path = &d[1..];
                let content = fs::read(path)
                    .map_err(|e| X402Error::General(format!("Failed to read file {}: {}", path, e)))?;
                Ok(Some(Body::from(content)))
            }
            Some(d) => Ok(Some(Body::from(d.clone()))),
            None => Ok(None),
        }
    }
}
```

**Step 2: Update src/main.rs to use request building**

```rust
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
```

**Step 3: Verify it works**

Run: `X402_PRIVATE_KEY=0xtest cargo run -- -X POST -H "Content-Type: application/json" -d '{"test":1}' -v https://example.com`
Expected: Shows method, URL, and headers

**Step 4: Commit**

```bash
git add src/request.rs src/main.rs
git commit -m "feat: add request building from CLI args"
```

---

## Task 5: Basic HTTP Client (without x402)

**Files:**
- Modify: `src/main.rs`
- Create: `src/output.rs`

**Step 1: Create src/output.rs**

```rust
use crate::error::X402Error;
use reqwest::Response;
use std::fs::File;
use std::io::Write;

pub async fn handle_response(
    response: Response,
    output_file: Option<&str>,
    fail_on_error: bool,
    verbose: bool,
) -> Result<(), X402Error> {
    let status = response.status();

    if verbose {
        eprintln!("< {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""));
    }

    if fail_on_error && !status.is_success() {
        return Err(X402Error::Http(format!("HTTP {} {}", status.as_u16(), status.canonical_reason().unwrap_or(""))));
    }

    let body = response.bytes().await?;

    match output_file {
        Some(path) => {
            let mut file = File::create(path)
                .map_err(|e| X402Error::General(format!("Failed to create output file: {}", e)))?;
            file.write_all(&body)
                .map_err(|e| X402Error::General(format!("Failed to write output file: {}", e)))?;
        }
        None => {
            // Write to stdout
            std::io::stdout()
                .write_all(&body)
                .map_err(|e| X402Error::General(format!("Failed to write to stdout: {}", e)))?;
            // Add newline if output doesn't end with one
            if !body.ends_with(b"\n") {
                println!();
            }
        }
    }

    Ok(())
}
```

**Step 2: Update src/main.rs with basic HTTP client**

```rust
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
```

**Step 3: Verify it works with a real request**

Run: `cargo run -- https://httpbin.org/get`
Expected: Returns JSON response from httpbin

Run: `cargo run -- -X POST -d '{"test":1}' -H "Content-Type: application/json" https://httpbin.org/post`
Expected: Returns POST response with body echoed

**Step 4: Commit**

```bash
git add src/output.rs src/main.rs
git commit -m "feat: add basic HTTP client with response handling"
```

---

## Task 6: x402 Payment Integration

**Files:**
- Modify: `src/main.rs`

**Step 1: Update src/main.rs with x402 client**

```rust
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
use x402_rs::client::{V1Eip155ExactClient, V2Eip155ExactClient};

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
    let response = request.send().await
        .map_err(|e| X402Error::Network(e.into()))?;

    // Handle response
    handle_response(
        response.into(),
        args.output.as_deref(),
        args.fail,
        verbose,
    ).await?;

    Ok(())
}
```

**Step 2: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 3: Test with non-402 endpoint**

Run: `X402_PRIVATE_KEY=0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80 cargo run -- https://httpbin.org/get`
Expected: Returns JSON response (key is Hardhat test key #0)

**Step 4: Commit**

```bash
git add src/main.rs Cargo.toml
git commit -m "feat: integrate x402-reqwest for automatic payments"
```

---

## Task 7: Dry-Run Mode

**Files:**
- Modify: `src/main.rs`

**Step 1: Add dry-run logic before x402 client**

Update the `run()` function to handle `--x402-dry-run`:

```rust
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

    // Dry-run mode: make request without payment handling
    if args.x402_dry_run {
        return dry_run(&req_config, verbose).await;
    }

    // ... rest of the function stays the same
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
                eprintln!("  Payment details: {}", serde_json::to_string_pretty(&json).unwrap_or(body));
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
```

**Step 2: Add serde_json to Cargo.toml**

```toml
serde_json = "1"
```

**Step 3: Verify it works**

Run: `cargo run -- --x402-dry-run https://httpbin.org/get`
Expected: Shows "< 200 (no payment required)" in verbose or completes silently

**Step 4: Commit**

```bash
git add src/main.rs Cargo.toml
git commit -m "feat: add dry-run mode to preview payment requirements"
```

---

## Task 8: Confirmation Mode

**Files:**
- Modify: `src/main.rs`

**Step 1: Add confirmation prompt function**

Add this function and update imports:

```rust
use std::io::{self, Write};

fn prompt_confirmation(amount: &str, recipient: &str) -> Result<bool, X402Error> {
    eprint!("Payment required: {}\nRecipient: {}\nProceed? [y/N] ", amount, recipient);
    io::stderr().flush().map_err(|e| X402Error::General(e.to_string()))?;

    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .map_err(|e| X402Error::General(e.to_string()))?;

    Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
}
```

**Note:** Full confirmation mode requires hooking into the x402-reqwest middleware. For the initial implementation, we'll add a pre-flight check that makes a request to check if payment is required, then prompts before making the actual paid request.

**Step 2: Add pre-flight confirmation check**

```rust
// Before making the x402 request, if --confirm is set:
if args.confirm || config.confirm {
    // Pre-flight request to check if payment required
    let preflight_client = reqwest::Client::new();
    let preflight_response = preflight_client
        .request(req_config.method.clone(), &req_config.url)
        .headers(req_config.headers.clone())
        .send()
        .await?;

    if preflight_response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        // Extract payment info and prompt
        let body = preflight_response.text().await.unwrap_or_default();
        let (amount, recipient) = parse_payment_info(&body);

        if !prompt_confirmation(&amount, &recipient)? {
            eprintln!("Payment cancelled.");
            return Ok(());
        }
    }
}
```

```rust
fn parse_payment_info(body: &str) -> (String, String) {
    // Try to parse JSON payment info
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        let amount = json.get("amount")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown amount");
        let recipient = json.get("recipient")
            .or_else(|| json.get("payTo"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        (amount.to_string(), recipient.to_string())
    } else {
        ("unknown amount".to_string(), "unknown".to_string())
    }
}
```

**Step 3: Verify it compiles**

Run: `cargo build`
Expected: Compiles successfully

**Step 4: Commit**

```bash
git add src/main.rs
git commit -m "feat: add confirmation mode for payment approval"
```

---

## Task 9: Form Data Support

**Files:**
- Modify: `src/request.rs`
- Modify: `src/main.rs`

**Step 1: Add form parsing to src/request.rs**

```rust
use reqwest::multipart::{Form, Part};

impl RequestConfig {
    // Add this method
    pub fn parse_form(form_fields: &[String]) -> Result<Option<Form>, X402Error> {
        if form_fields.is_empty() {
            return Ok(None);
        }

        let mut form = Form::new();
        for field in form_fields {
            let parts: Vec<&str> = field.splitn(2, '=').collect();
            if parts.len() != 2 {
                return Err(X402Error::General(format!("Invalid form field: {}", field)));
            }
            let name = parts[0];
            let value = parts[1];

            if value.starts_with('@') {
                // File upload
                let path = &value[1..];
                let filename = std::path::Path::new(path)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("file")
                    .to_string();
                let content = std::fs::read(path)
                    .map_err(|e| X402Error::General(format!("Failed to read file {}: {}", path, e)))?;
                let part = Part::bytes(content).file_name(filename);
                form = form.part(name.to_string(), part);
            } else {
                form = form.text(name.to_string(), value.to_string());
            }
        }
        Ok(Some(form))
    }
}
```

**Step 2: Update src/main.rs to handle form data**

In the request building section, add form handling:

```rust
// Build request
let mut request = client.request(req_config.method.clone(), &req_config.url);
request = request.headers(req_config.headers.clone());

// Form data takes precedence over body
if let Some(form) = RequestConfig::parse_form(&args.form)? {
    request = request.multipart(form);
} else if let Some(body) = req_config.body {
    request = request.body(body);
}
```

**Step 3: Verify it works**

Run: `X402_PRIVATE_KEY=0xtest cargo run -- -F "name=test" -F "file=@Cargo.toml" https://httpbin.org/post`
Expected: Returns response showing multipart form data

**Step 4: Commit**

```bash
git add src/request.rs src/main.rs
git commit -m "feat: add multipart form data support"
```

---

## Task 10: Final Integration & Testing

**Files:**
- Create: `tests/integration.rs`
- Modify: `src/main.rs` (final cleanup)

**Step 1: Create tests/integration.rs**

```rust
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
```

**Step 2: Run tests**

Run: `cargo test`
Expected: All tests pass

**Step 3: Final commit**

```bash
git add tests/
git commit -m "test: add integration tests for CLI functionality"
```

---

## Task 11: Documentation

**Files:**
- Create: `README.md`

**Step 1: Create README.md**

```markdown
# x402curl

A curl-like CLI tool with automatic x402 payment handling.

## Installation

```bash
cargo install x402curl
```

## Configuration

Set your private key using one of these methods (in priority order):

1. **CLI flag**: `--x402-key 0x...`
2. **Environment variable**: `export X402_PRIVATE_KEY=0x...`
3. **Local .env file**: Add `X402_PRIVATE_KEY=0x...` to `./.env`
4. **Global config**: Create `~/.x402/config`:
   ```toml
   private_key = "0x..."
   ```

## Usage

```bash
# Basic GET request
x402curl https://api.example.com/resource

# POST with JSON body
x402curl -X POST -H "Content-Type: application/json" -d '{"key": "value"}' https://api.example.com

# Upload file
x402curl -F "file=@document.pdf" https://api.example.com/upload

# Preview payment requirements without paying
x402curl --x402-dry-run https://api.example.com/paid-endpoint

# Require confirmation before payment
x402curl --confirm https://api.example.com/paid-endpoint

# Verbose mode (shows payment details)
x402curl -v https://api.example.com/paid-endpoint
```

## Supported Flags

| Flag | Description |
|------|-------------|
| `-X, --request` | HTTP method |
| `-H, --header` | Add header |
| `-d, --data` | Request body |
| `-o, --output` | Output file |
| `-f, --fail` | Fail on HTTP errors |
| `-s, --silent` | Silent mode |
| `-v, --verbose` | Verbose output |
| `-F, --form` | Form field |
| `-u, --user` | Basic auth |
| `-L, --location` | Follow redirects |
| `--x402-dry-run` | Preview payment |
| `--confirm` | Confirm before paying |

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Network error |
| 3 | Payment error |
| 4 | HTTP error |
| 5 | Config error |

## License

MIT
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add README with usage instructions"
```

---

## Summary

This plan creates x402curl in 11 tasks:

1. **Project Skeleton** — Cargo.toml, main.rs, error types
2. **CLI Parsing** — clap-based argument handling
3. **Configuration** — Private key loading hierarchy
4. **Request Building** — Headers, body, method parsing
5. **Basic HTTP Client** — reqwest without x402
6. **x402 Integration** — Add x402-reqwest middleware
7. **Dry-Run Mode** — Preview payments without paying
8. **Confirmation Mode** — Prompt before paying
9. **Form Data** — Multipart upload support
10. **Integration Tests** — CLI test coverage
11. **Documentation** — README with examples
