pub mod download;
pub mod list;
pub mod sync;
pub mod upload;

use anyhow::{Context, Result};

use crate::{api::SolidropApi, config::CliConfig};

/// Loaded context shared across all commands.
pub struct CmdContext {
    pub api: SolidropApi,
    pub master_key: [u8; 32],
    pub config: CliConfig,
}

impl CmdContext {
    pub fn load() -> Result<Self> {
        let config = CliConfig::load().map_err(|e| anyhow::anyhow!("{e}"))?;

        let api_key = std::env::var(&config.server.api_key_env).with_context(|| {
            format!(
                "API key env var `{}` is not set (configured in [server] api_key_env)",
                config.server.api_key_env
            )
        })?;

        let api = SolidropApi::new(config.server.endpoint.clone(), api_key);
        let master_key = crate::key::load_master_key()?;

        Ok(Self {
            api,
            master_key,
            config,
        })
    }
}
