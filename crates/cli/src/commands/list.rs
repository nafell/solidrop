pub async fn run(prefix: Option<&str>) -> anyhow::Result<()> {
    tracing::info!("listing files with prefix: {:?}", prefix);
    // TODO: Implement list flow:
    // 1. Call GET /api/v1/files with optional prefix
    // 2. Display file list (path, size, date)
    println!("list not yet implemented");
    Ok(())
}
