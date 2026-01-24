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
