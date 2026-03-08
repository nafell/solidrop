use super::CmdContext;

pub async fn run(from: &str, to: &str) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;
    ctx.api.move_file(from, to).await?;
    println!("Moved: {from} -> {to}");
    Ok(())
}
