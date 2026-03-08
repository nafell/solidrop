use anyhow::Context;

use super::{download, CmdContext};

/// Download all remote files that are not already present in the local download directory.
///
/// Note: This is a simple MVP implementation that checks file existence only.
/// Full LRU cache management and local SQLite state are planned for a future milestone.
pub async fn run() -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    println!("Fetching remote file list...");

    // Collect all pages before iterating
    let mut all_files = Vec::new();
    let mut next_token: Option<String> = None;
    loop {
        let resp = ctx
            .api
            .list_files(None, next_token.as_deref())
            .await
            .context("failed to list remote files")?;
        all_files.extend(resp.files);
        next_token = resp.next_token;
        if next_token.is_none() {
            break;
        }
    }

    if all_files.is_empty() {
        println!("No remote files found.");
        return Ok(());
    }

    println!(
        "Found {} remote file(s). Checking local state...",
        all_files.len()
    );

    let download_dir = &ctx.config.storage.download_dir;
    let mut downloaded = 0usize;
    let mut skipped = 0usize;

    for file in &all_files {
        let local_filename = std::path::Path::new(&file.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or(&file.path)
            .strip_suffix(".enc")
            .unwrap_or(&file.path);

        let local_path = download_dir.join(local_filename);

        if local_path.exists() {
            tracing::debug!(path = %file.path, "already local, skipping");
            skipped += 1;
            continue;
        }

        // Re-use the download command's logic (includes integrity verification)
        download::run(&file.path).await?;
        downloaded += 1;
    }

    println!(
        "Sync complete: {} downloaded, {} already present.",
        downloaded, skipped
    );

    Ok(())
}
