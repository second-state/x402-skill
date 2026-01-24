use std::process::ExitCode;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum X402Error {
    #[error("General error: {0}")]
    General(String),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("Middleware error: {0}")]
    Middleware(#[from] reqwest_middleware::Error),

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
            X402Error::Middleware(_) => ExitCode::from(2),
            X402Error::Payment(_) => ExitCode::from(3),
            X402Error::Http(_) => ExitCode::from(4),
            X402Error::Config(_) => ExitCode::from(5),
        }
    }

    pub fn code_number(&self) -> u8 {
        match self {
            X402Error::General(_) => 1,
            X402Error::Network(_) => 2,
            X402Error::Middleware(_) => 2,
            X402Error::Payment(_) => 3,
            X402Error::Http(_) => 4,
            X402Error::Config(_) => 5,
        }
    }
}
