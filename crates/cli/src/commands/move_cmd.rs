use anyhow::Result;

use crate::api_client::ApiClient;

pub async fn run(api: &ApiClient, from: &str, to: &str) -> Result<()> {
    api.move_file(from, to).await?;
    println!("Moved: {} -> {}", from, to);
    Ok(())
}
