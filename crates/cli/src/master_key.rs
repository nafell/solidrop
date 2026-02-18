use anyhow::{Context, Result};

use crate::config::CryptoConfig;

const MASTER_KEY_ENV: &str = "SOLIDROP_MASTER_KEY";

/// Acquire the 32-byte master key from the environment.
///
/// MVP implementation reads from the `SOLIDROP_MASTER_KEY` environment variable
/// (hex-encoded, 64 characters = 32 bytes). Future versions will support OS keychain.
pub fn acquire_master_key(_config: &CryptoConfig) -> Result<[u8; 32]> {
    let hex_key = std::env::var(MASTER_KEY_ENV).with_context(|| {
        format!(
            "environment variable '{MASTER_KEY_ENV}' is not set.\n\
             \n\
             Generate a key with:\n  \
             openssl rand -hex 32\n\
             \n\
             Then export it:\n  \
             export {MASTER_KEY_ENV}=<your-64-char-hex-key>"
        )
    })?;

    let bytes = hex::decode(&hex_key).with_context(|| {
        format!("{MASTER_KEY_ENV} is not valid hex (expected 64 hex characters)")
    })?;

    let key: [u8; 32] = bytes.try_into().map_err(|v: Vec<u8>| {
        anyhow::anyhow!(
            "{MASTER_KEY_ENV} must be exactly 32 bytes (64 hex chars), got {} bytes ({} hex chars)",
            v.len(),
            v.len() * 2
        )
    })?;

    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CryptoConfig;

    fn dummy_config() -> CryptoConfig {
        CryptoConfig {
            keychain_service: "test".into(),
            keychain_account: "test".into(),
        }
    }

    #[test]
    fn test_valid_master_key() {
        let key_hex = "aa".repeat(32); // 64 hex chars = 32 bytes
        std::env::set_var(MASTER_KEY_ENV, &key_hex);
        let result = acquire_master_key(&dummy_config());
        std::env::remove_var(MASTER_KEY_ENV);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), [0xaa; 32]);
    }

    #[test]
    fn test_missing_env_var() {
        std::env::remove_var(MASTER_KEY_ENV);
        let result = acquire_master_key(&dummy_config());
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not set"), "error: {err}");
    }

    #[test]
    fn test_invalid_hex() {
        std::env::set_var(MASTER_KEY_ENV, "not-valid-hex!");
        let result = acquire_master_key(&dummy_config());
        std::env::remove_var(MASTER_KEY_ENV);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("not valid hex"), "error: {err}");
    }

    #[test]
    fn test_wrong_length() {
        std::env::set_var(MASTER_KEY_ENV, "aabb"); // 2 bytes, not 32
        let result = acquire_master_key(&dummy_config());
        std::env::remove_var(MASTER_KEY_ENV);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("32 bytes"), "error: {err}");
    }
}
