use anyhow::Context;

use super::CmdContext;

pub async fn run(prefix: Option<&str>) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    let resp = ctx
        .api
        .list_files(prefix)
        .await
        .context("failed to list files")?;

    if resp.files.is_empty() {
        println!("No files found.");
        return Ok(());
    }

    // Column widths
    let path_width = resp.files.iter().map(|f| f.path.len()).max().unwrap_or(4).max(4);

    println!(
        "{:<path_width$}  {:>12}  {}",
        "PATH",
        "SIZE",
        "LAST MODIFIED",
        path_width = path_width,
    );
    println!("{}", "-".repeat(path_width + 2 + 12 + 2 + 24));

    for file in &resp.files {
        println!(
            "{:<path_width$}  {:>12}  {}",
            file.path,
            format_size(file.size_bytes),
            file.last_modified,
            path_width = path_width,
        );
    }

    if resp.next_token.is_some() {
        println!("(more results available; pagination not yet supported in CLI)");
    }

    Ok(())
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;

    if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.1} KB", bytes as f64 / KB as f64)
    } else {
        format!("{bytes} B")
    }
}
