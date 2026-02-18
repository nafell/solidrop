use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct CliConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub crypto: CryptoConfig,
}

#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    pub endpoint: String,
    pub api_key_env: String,
}

#[derive(Debug, Deserialize)]
pub struct StorageConfig {
    pub download_dir: PathBuf,
    pub upload_dir: PathBuf,
}

#[derive(Debug, Deserialize)]
pub struct CryptoConfig {
    pub keychain_service: String,
    pub keychain_account: String,
}

impl CliConfig {
    pub fn load() -> Result<Self> {
        let config_dir = directories::ProjectDirs::from("dev", "nafell", "solidrop")
            .context("could not determine config directory")?;
        let config_path = config_dir.config_dir().join("config.toml");
        let content = std::fs::read_to_string(&config_path)
            .with_context(|| format!("could not read config file at {}", config_path.display()))?;
        let config: CliConfig = toml::from_str(&content)
            .with_context(|| format!("failed to parse config file at {}", config_path.display()))?;
        Ok(config)
    }
}
