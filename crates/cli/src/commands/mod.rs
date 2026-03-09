pub mod delete;
pub mod download;
pub mod list;
pub mod move_cmd;
pub mod print_verifier;
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
        let master_key = crate::master_key::acquire_master_key(&config.crypto)?;

        // Derive the API token from the master key (ADR-003).
        // The server verifies SHA-256(token); the token itself is never stored server-side.
        let api_token = solidrop_crypto::key_derivation::derive_api_token(&master_key)
            .context("failed to derive API token from master key")?;
        let api_token_hex = hex::encode(api_token);

        let api = SolidropApi::new(config.server.endpoint.clone(), api_token_hex);

        Ok(Self {
            api,
            master_key,
            config,
        })
    }
}
