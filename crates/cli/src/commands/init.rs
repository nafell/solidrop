use anyhow::{bail, Context, Result};
use solidrop_crypto::{hash::sha256_raw, key_derivation::{derive_api_token, derive_master_key, generate_salt}};

/// Set up the master password for the first time (or reset it).
///
/// Prompts for a new password twice, generates a random Argon2id salt,
/// derives the master key, saves the salt to `~/.config/solidrop/master.salt`,
/// and prints the `SOLIDROP_API_KEY_VERIFIER_SHA256` value to set on the server.
///
/// The salt is not secret; only the password is secret and is never stored.
pub fn run() -> Result<()> {
    let salt_path = crate::key::salt_path()?;

    if salt_path.exists() {
        eprintln!(
            "Warning: master.salt already exists at {}.",
            salt_path.display()
        );
        eprintln!("Proceeding will replace the existing key derivation salt.");
        eprintln!("All previously encrypted data will become unrecoverable without the old password + old salt.");
        eprintln!();
    }

    let password =
        rpassword::prompt_password("New password: ").context("failed to read password")?;
    if password.is_empty() {
        bail!("Password must not be empty");
    }
    let confirm =
        rpassword::prompt_password("Confirm password: ").context("failed to read password")?;
    if password != confirm {
        bail!("Passwords do not match");
    }

    let salt = generate_salt();
    let master_key = derive_master_key(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("key derivation failed: {e}"))?;

    // Persist salt (not secret — Argon2id security comes from the password)
    if let Some(parent) = salt_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create config dir at {}", parent.display()))?;
    }
    std::fs::write(&salt_path, &salt)
        .with_context(|| format!("failed to write salt to {}", salt_path.display()))?;

    // Derive the API token verifier for server-side configuration (ADR-003)
    let api_token = derive_api_token(&master_key)
        .map_err(|e| anyhow::anyhow!("failed to derive API token: {e}"))?;
    let verifier_hex = hex::encode(sha256_raw(&api_token));

    println!("Initialized successfully. Salt saved to: {}", salt_path.display());
    println!();
    println!("SOLIDROP_API_KEY_VERIFIER_SHA256={verifier_hex}");
    println!();
    println!("Set this value on the server (e.g. /etc/solidrop/staging.env).");

    Ok(())
}
