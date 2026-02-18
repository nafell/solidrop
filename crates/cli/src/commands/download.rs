use anyhow::{Context, Result};
use std::path::Path;

use crate::api_client::ApiClient;
use crate::config::CliConfig;

pub async fn run(
    config: &CliConfig,
    api: &ApiClient,
    key: &[u8; 32],
    remote_path: &str,
) -> Result<()> {
    let download_url = api.presign_download(remote_path).await?;
    let encrypted_data = api.get_from_s3(&download_url).await?;

    let plaintext =
        solidrop_crypto::decrypt::decrypt(key, &encrypted_data).context("decryption failed")?;

    let basename = Path::new(remote_path)
        .file_name()
        .context("invalid remote path: no filename")?
        .to_str()
        .context("filename is not valid UTF-8")?;
    let filename = basename.strip_suffix(".enc").unwrap_or(basename);

    let output_dir = &config.storage.download_dir;
    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create download directory: {}",
            output_dir.display()
        )
    })?;

    let output_path = output_dir.join(filename);
    std::fs::write(&output_path, &plaintext)
        .with_context(|| format!("failed to write file: {}", output_path.display()))?;

    println!("Downloaded: {} -> {}", remote_path, output_path.display());
    Ok(())
}
