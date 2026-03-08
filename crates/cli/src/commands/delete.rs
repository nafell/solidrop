use super::CmdContext;

pub async fn run(remote_path: &str) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;
    ctx.api.delete_file(remote_path).await?;
    println!("Deleted: {remote_path}");
    Ok(())
}
