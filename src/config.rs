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
        if dotenvy::dotenv().is_ok() {
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
