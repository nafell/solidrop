use anyhow::{Context, Result};
use std::path::Path;

use solidrop_crypto::decrypt::decrypt;

use super::CmdContext;

pub async fn run(remote_path: &str) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    println!("Requesting presigned download URL for {remote_path}...");
    let presign = ctx
        .api
        .presign_download(remote_path)
        .await
        .context("failed to get presigned download URL")?;

    println!("Downloading from S3...");
    let ciphertext = ctx
        .api
        .get_object(&presign.url)
        .await
        .context("download from S3 failed")?;

    println!("Decrypting ({} bytes)...", ciphertext.len());
    let plaintext = decrypt(&ctx.master_key, &ciphertext).context("decryption failed")?;

    // Derive local filename: strip trailing ".enc" if present
    let local_filename = Path::new(remote_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or(remote_path)
        .strip_suffix(".enc")
        .unwrap_or(remote_path);

    let download_dir = &ctx.config.storage.download_dir;
    std::fs::create_dir_all(download_dir)
        .with_context(|| format!("failed to create download dir: {}", download_dir.display()))?;

    let dest = download_dir.join(local_filename);
    std::fs::write(&dest, &plaintext)
        .with_context(|| format!("failed to write file: {}", dest.display()))?;

    println!("✓ Downloaded {remote_path} → {} ({} bytes)", dest.display(), plaintext.len());
    Ok(())
}
