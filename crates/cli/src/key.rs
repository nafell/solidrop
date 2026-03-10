use anyhow::{Context, Result};
use std::path::PathBuf;

/// Returns the path to the stored Argon2id salt file.
///
/// The salt is generated once by `solidrop init` and persisted here.
/// It is not secret — only the password is secret.
pub(crate) fn salt_path() -> Result<PathBuf> {
    let dirs = directories::ProjectDirs::from("dev", "nafell", "solidrop")
        .context("could not determine config directory")?;
    Ok(dirs.config_dir().join("master.salt"))
}

/// Derive the 32-byte master key from the stored salt and a password prompt.
///
/// Reads the Argon2id salt from `~/.config/solidrop/master.salt`, prompts the
/// user for their password (input hidden), and derives the master key using
/// `solidrop_crypto::key_derivation::derive_master_key`.
///
/// Run `solidrop init` once to generate and save the salt.
pub fn load_master_key() -> Result<[u8; 32]> {
    let path = salt_path()?;
    let salt_bytes = std::fs::read(&path).with_context(|| {
        format!(
            "master.salt not found at {}.\nRun `solidrop init` to set up your password.",
            path.display()
        )
    })?;
    let salt: [u8; 16] = salt_bytes.try_into().map_err(|v: Vec<u8>| {
        anyhow::anyhow!(
            "master.salt is corrupted: expected 16 bytes, got {}",
            v.len()
        )
    })?;

    let password =
        rpassword::prompt_password("Password: ").context("failed to read password")?;

    solidrop_crypto::key_derivation::derive_master_key(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("key derivation failed: {e}"))
}
