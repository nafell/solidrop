use anyhow::Result;

use crate::api_client::ApiClient;

pub async fn run(api: &ApiClient, remote_path: &str) -> Result<()> {
    api.delete_file(remote_path).await?;
    println!("Deleted: {}", remote_path);
    Ok(())
}
