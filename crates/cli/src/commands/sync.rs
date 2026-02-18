use anyhow::{Context, Result};
use std::path::Path;

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
            let basename = Path::new(&file.key)
                .file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            let filename = basename.strip_suffix(".enc").unwrap_or(&basename);

            let local_path = config.storage.download_dir.join(filename);
            if local_path.exists() {
                skipped += 1;
                continue;
            }

            let download_url = api.presign_download(&file.key).await?;
            let encrypted_data = api.get_from_s3(&download_url).await?;
            let plaintext = solidrop_crypto::decrypt::decrypt(key, &encrypted_data)
                .context("decryption failed")?;

            std::fs::create_dir_all(&config.storage.download_dir).with_context(|| {
                format!(
                    "failed to create download directory: {}",
                    config.storage.download_dir.display()
                )
            })?;

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
