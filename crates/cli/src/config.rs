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
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_dir = directories::ProjectDirs::from("com", "artsync", "art-sync")
            .ok_or("could not determine config directory")?;
        let config_path = config_dir.config_dir().join("config.toml");
        let content = std::fs::read_to_string(&config_path).map_err(|e| {
            format!(
                "could not read config file at {}: {e}",
                config_path.display()
            )
        })?;
        let config: CliConfig = toml::from_str(&content)?;
        Ok(config)
    }
}
