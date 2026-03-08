use anyhow::{bail, Context};
use std::path::Path;

use solidrop_crypto::{encrypt::encrypt, hash::sha256_hex};

use super::CmdContext;

pub async fn run(file_path: &str, remote_path_override: Option<&str>) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    let path = Path::new(file_path);
    if !path.exists() {
        bail!("file not found: {file_path}");
    }

    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .with_context(|| format!("invalid filename in path: {file_path}"))?;

    // Use caller-supplied remote path, or default to "<filename>.enc"
    let remote_path = match remote_path_override {
        Some(rp) => rp.to_string(),
        None => format!("{filename}.enc"),
    };

    println!("Reading {file_path}...");
    let plaintext = std::fs::read(path).with_context(|| format!("failed to read {file_path}"))?;

    // SHA-256 hash of the plaintext (before encryption) for integrity verification
    let content_hash = sha256_hex(&plaintext);
    tracing::debug!(hash = %content_hash, "computed content hash");

    println!("Encrypting ({} bytes)...", plaintext.len());
    let ciphertext = encrypt(&ctx.master_key, &plaintext).context("encryption failed")?;

    println!("Requesting presigned upload URL for {remote_path}...");
    let presign = ctx
        .api
        .presign_upload(&remote_path, &content_hash, ciphertext.len() as u64)
        .await
        .context("failed to get presigned upload URL")?;

    println!("Uploading {} bytes to S3...", ciphertext.len());
    ctx.api
        .put_object(&presign.url, ciphertext, Some(&content_hash))
        .await
        .context("upload to S3 failed")?;

    println!("Uploaded {filename} -> {remote_path}");
    Ok(())
}
