pub async fn run() -> anyhow::Result<()> {
    tracing::info!("syncing new files");
    // TODO: Implement sync flow:
    // 1. Call GET /api/v1/files to get remote file list
    // 2. Compare with local state
    // 3. Download new/updated files
    println!("sync not yet implemented");
    Ok(())
}
