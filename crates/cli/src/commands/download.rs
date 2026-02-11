pub async fn run(remote_path: &str) -> anyhow::Result<()> {
    tracing::info!("downloading {remote_path}");
    // TODO: Implement download flow:
    // 1. Request presigned download URL from API server
    // 2. GET encrypted data from S3 via presigned URL
    // 3. Decrypt with AES-256-GCM
    // 4. Verify SHA-256 hash
    // 5. Save to local download directory
    println!("download not yet implemented");
    Ok(())
}
