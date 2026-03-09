use anyhow::{Context, Result};

/// Derive and print the API key verifier for server-side configuration.
///
/// Reads `SOLIDROP_MASTER_KEY` from the environment, derives the API token
/// via HKDF, then prints `hex(SHA-256(api_token))`.
///
/// The output value must be set as `SOLIDROP_API_KEY_VERIFIER_SHA256` on the
/// API server. See ADR-003 for the full auth design.
pub fn run() -> Result<()> {
    let master_key = crate::master_key::acquire_master_key(&crate::config::CryptoConfig {
        keychain_service: String::new(),
        keychain_account: String::new(),
    })?;

    let api_token = solidrop_crypto::key_derivation::derive_api_token(&master_key)
        .context("failed to derive API token from master key")?;

    let verifier = solidrop_crypto::hash::sha256_raw(&api_token);
    let verifier_hex = hex::encode(verifier);

    println!("SOLIDROP_API_KEY_VERIFIER_SHA256={verifier_hex}");
    println!();
    println!("Set this value on the server (e.g. /etc/solidrop/staging.env).");

    Ok(())
}
