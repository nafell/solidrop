use anyhow::Context;
use std::path::Path;

use solidrop_crypto::decrypt::decrypt;

use super::CmdContext;

/// Returns true if the SHA-256 hex digest of `plaintext` matches `expected`.
pub fn verify_content_hash(plaintext: &[u8], expected: &str) -> bool {
    solidrop_crypto::hash::sha256_hex(plaintext) == expected
}

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

    // Verify integrity against the hash stored in S3 object metadata
    if let Some(expected_hash) = &presign.content_hash {
        if !verify_content_hash(&plaintext, expected_hash) {
            anyhow::bail!("integrity check failed: content_hash mismatch for {remote_path}");
        }
        tracing::debug!("integrity check passed");
    }

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

    println!(
        "✓ Downloaded {remote_path} → {} ({} bytes)",
        dest.display(),
        plaintext.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_content_hash;

    #[test]
    fn correct_hash_passes() {
        let data = b"hello world";
        // SHA-256 of "hello world"
        // Use the actual function from the crypto crate
        let actual = solidrop_crypto::hash::sha256_hex(data);
        assert!(verify_content_hash(data, &actual));
        // Also verify a known-bad hash fails
        assert!(!verify_content_hash(data, "deadbeef"));
    }

    #[test]
    fn wrong_hash_fails() {
        let data = b"some content";
        assert!(!verify_content_hash(
            data,
            "0000000000000000000000000000000000000000000000000000000000000000"
        ));
    }
}
