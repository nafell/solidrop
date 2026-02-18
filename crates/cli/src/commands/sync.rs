use anyhow::{Context, Result};

use crate::api_client::ApiClient;
use crate::config::CliConfig;

pub async fn run(config: &CliConfig, api: &ApiClient, key: &[u8; 32]) -> Result<()> {
    let mut downloaded = 0u64;
    let mut skipped = 0u64;
    let mut next_token: Option<String> = None;

    loop {
        let (files, token) = api
            .list_files(Some("transfer/"), None, next_token.as_deref())
            .await?;

        for file in &files {
            // Preserve directory structure: strip "transfer/" prefix, then strip ".enc" suffix.
            // e.g. "transfer/2026-02-11/reference.png.enc" -> "2026-02-11/reference.png"
            let relative = file.key.strip_prefix("transfer/").unwrap_or(&file.key);
            let relative = relative.strip_suffix(".enc").unwrap_or(relative);

            let local_path = config.storage.download_dir.join(relative);
            if local_path.exists() {
                skipped += 1;
                continue;
            }

            let download_url = api.presign_download(&file.key).await?;
            let encrypted_data = api.get_from_s3(&download_url).await?;
            let plaintext = solidrop_crypto::decrypt::decrypt(key, &encrypted_data)
                .context("decryption failed")?;

            // Create parent directories (e.g. download_dir/2026-02-11/)
            if let Some(parent) = local_path.parent() {
                std::fs::create_dir_all(parent)
                    .with_context(|| format!("failed to create directory: {}", parent.display()))?;
            }

            std::fs::write(&local_path, &plaintext)
                .with_context(|| format!("failed to write file: {}", local_path.display()))?;

            downloaded += 1;
        }

        next_token = token;
        if next_token.is_none() {
            break;
        }
    }

    println!(
        "Sync complete: {} downloaded, {} skipped",
        downloaded, skipped
    );
    Ok(())
}
