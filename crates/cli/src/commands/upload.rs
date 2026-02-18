use anyhow::{Context, Result};
use chrono::Utc;
use std::path::Path;

use crate::api_client::ApiClient;
use crate::config::CliConfig;

pub async fn run(
    _config: &CliConfig,
    api: &ApiClient,
    key: &[u8; 32],
    file_path: &str,
) -> Result<()> {
    let path = Path::new(file_path);
    let filename = path
        .file_name()
        .context("invalid file path: no filename")?
        .to_str()
        .context("filename is not valid UTF-8")?;

    let plaintext =
        std::fs::read(path).with_context(|| format!("failed to read file: {}", file_path))?;

    let content_hash = solidrop_crypto::hash::sha256_hex(&plaintext);
    let ciphertext =
        solidrop_crypto::encrypt::encrypt(key, &plaintext).context("encryption failed")?;

    let now = Utc::now();
    let remote_path = format!("active/{}/{}.enc", now.format("%Y-%m"), filename);

    let upload_url = api
        .presign_upload(&remote_path, &content_hash, ciphertext.len() as u64)
        .await?;
    api.put_to_s3(&upload_url, &ciphertext).await?;

    println!(
        "Uploaded: {} -> {} ({} bytes)",
        file_path,
        remote_path,
        ciphertext.len()
    );
    Ok(())
}
