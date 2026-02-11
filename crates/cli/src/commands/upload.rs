pub async fn run(file_path: &str) -> anyhow::Result<()> {
    tracing::info!("uploading {file_path}");
    // TODO: Implement upload flow:
    // 1. Read file
    // 2. Compute SHA-256 hash
    // 3. Encrypt with AES-256-GCM
    // 4. Request presigned upload URL from API server
    // 5. PUT encrypted data to S3 via presigned URL
    println!("upload not yet implemented");
    Ok(())
}
