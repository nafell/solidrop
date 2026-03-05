use anyhow::Context;

use super::{download, CmdContext};

/// Download all remote files that are not already present in the local download directory.
///
/// Note: This is a simple MVP implementation that checks file existence only.
/// Full LRU cache management and local SQLite state are planned for a future milestone.
pub async fn run() -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    println!("Fetching remote file list...");
    let resp = ctx
        .api
        .list_files(None)
        .await
        .context("failed to list remote files")?;

    if resp.files.is_empty() {
        println!("No remote files found.");
        return Ok(());
    }

    println!("Found {} remote file(s). Checking local state...", resp.files.len());

    let download_dir = &ctx.config.storage.download_dir;
    let mut downloaded = 0usize;
    let mut skipped = 0usize;

    for file in &resp.files {
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

        // Re-use the download command's logic
        download::run(&file.path).await?;
        downloaded += 1;
    }

    println!(
        "Sync complete: {} downloaded, {} already present.",
        downloaded, skipped
    );

    Ok(())
}
