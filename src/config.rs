use crate::error::X402Error;
use alloy_signer_local::PrivateKeySigner;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Default)]
pub struct Config {
    pub private_key: Option<String>,
    pub wallet_path: Option<String>,
    pub wallet_password: Option<String>,
    pub verbose: bool,
    pub confirm: bool,
}

#[derive(Debug, serde::Deserialize)]
struct ConfigFile {
    private_key: Option<String>,
    wallet_path: Option<String>,
    wallet_password: Option<String>,
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
    /// 1. --x402-key CLI flag
    /// 2. X402_PRIVATE_KEY env var
    /// 3. .env file X402_PRIVATE_KEY
    /// 4. --x402-wallet + --x402-wallet-password CLI flags
    /// 5. X402_WALLET + X402_WALLET_PASSWORD env vars (including .env)
    /// 6. ~/.x402/config
    pub fn load(
        cli_key: Option<&str>,
        cli_wallet: Option<&str>,
        cli_wallet_password: Option<&str>,
    ) -> Result<Self, X402Error> {
        // Priority 1: CLI private key flag
        if let Some(key) = cli_key {
            return Ok(Config {
                private_key: Some(key.to_string()),
                ..Default::default()
            });
        }

        // Priority 2: X402_PRIVATE_KEY env var
        if let Ok(key) = std::env::var("X402_PRIVATE_KEY") {
            if !key.is_empty() {
                return Ok(Config {
                    private_key: Some(key),
                    ..Default::default()
                });
            }
        }

        // Priority 3: .env file -> X402_PRIVATE_KEY
        let _ = dotenvy::dotenv();
        if let Ok(key) = std::env::var("X402_PRIVATE_KEY") {
            if !key.is_empty() {
                return Ok(Config {
                    private_key: Some(key),
                    ..Default::default()
                });
            }
        }

        // Priority 4: CLI wallet flags
        if let Some(wallet) = cli_wallet {
            return Ok(Config {
                wallet_path: Some(wallet.to_string()),
                wallet_password: cli_wallet_password.map(|p| p.to_string()),
                ..Default::default()
            });
        }

        // Priority 5: X402_WALLET env var (including from .env loaded above)
        if let Ok(wallet) = std::env::var("X402_WALLET") {
            if !wallet.is_empty() {
                let password = std::env::var("X402_WALLET_PASSWORD").ok();
                return Ok(Config {
                    wallet_path: Some(wallet),
                    wallet_password: password,
                    ..Default::default()
                });
            }
        }

        // Priority 6: ~/.x402/config
        if let Some(config_path) = Self::global_config_path() {
            if config_path.exists() {
                return Self::load_from_file(&config_path);
            }
        }

        // No credentials found
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
            wallet_path: config_file.wallet_path,
            wallet_password: config_file.wallet_password,
            verbose: config_file.defaults.verbose,
            confirm: config_file.defaults.confirm,
        })
    }

    /// Build a PrivateKeySigner from whichever credentials are available.
    /// Priority: private_key first, then wallet keystore.
    pub fn require_signer(&self) -> Result<PrivateKeySigner, X402Error> {
        if let Some(ref key) = self.private_key {
            return key
                .parse()
                .map_err(|e| X402Error::Config(format!("Invalid private key: {}", e)));
        }

        if let Some(ref wallet_path) = self.wallet_path {
            let password = self.wallet_password.as_deref().ok_or_else(|| {
                X402Error::Config(
                    "Wallet keystore specified but no password provided. \
                     Use --x402-wallet-password or set X402_WALLET_PASSWORD."
                        .to_string(),
                )
            })?;

            let path = Path::new(wallet_path);
            if !path.exists() {
                return Err(X402Error::Config(format!(
                    "Wallet keystore file not found: {}",
                    wallet_path
                )));
            }

            return PrivateKeySigner::decrypt_keystore(path, password).map_err(|e| {
                X402Error::Config(format!(
                    "Failed to decrypt keystore '{}': {}",
                    wallet_path, e
                ))
            });
        }

        Err(X402Error::Config(
            "No wallet credentials found. Set X402_PRIVATE_KEY, use --x402-key, \
             or provide a keystore with --x402-wallet."
                .to_string(),
        ))
    }
}
