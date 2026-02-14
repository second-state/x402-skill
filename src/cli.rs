use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "x402curl")]
#[command(about = "curl with automatic x402 payment handling")]
#[command(version)]
pub struct Args {
    /// URL to request
    #[arg(required_unless_present = "x402_balance")]
    pub url: Option<String>,

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

    /// Path to Ethereum keystore (wallet.json) file
    #[arg(long = "x402-wallet")]
    pub x402_wallet: Option<String>,

    /// Password for the keystore wallet file
    #[arg(long = "x402-wallet-password")]
    pub x402_wallet_password: Option<String>,

    /// Show payment requirements without paying
    #[arg(long = "x402-dry-run")]
    pub x402_dry_run: bool,

    /// Prompt before making payment
    #[arg(long = "confirm")]
    pub confirm: bool,

    /// Query wallet USDC balance on Base network
    #[arg(long = "x402-balance")]
    pub x402_balance: bool,

    /// Override RPC endpoint URL (default: https://mainnet.base.org)
    #[arg(long = "x402-rpc-url")]
    pub x402_rpc_url: Option<String>,

    /// Override ERC-20 token contract address (default: USDC on detected chain)
    #[arg(long = "x402-token")]
    pub x402_token: Option<String>,
}

impl Args {
    pub fn parse_args() -> Self {
        Args::parse()
    }
}
