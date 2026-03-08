use anyhow::Context;

use super::CmdContext;

pub async fn run(prefix: Option<&str>) -> anyhow::Result<()> {
    let ctx = CmdContext::load()?;

    // Collect all pages before displaying
    let mut all_files = Vec::new();
    let mut next_token: Option<String> = None;
    loop {
        let resp = ctx
            .api
            .list_files(prefix, next_token.as_deref())
            .await
            .context("failed to list files")?;
        all_files.extend(resp.files);
        next_token = resp.next_token;
        if next_token.is_none() {
            break;
        }
    }

    if all_files.is_empty() {
        println!("No files found.");
        return Ok(());
    }

    // Column widths
    let path_width = all_files
        .iter()
        .map(|f| f.path.len())
        .max()
        .unwrap_or(4)
        .max(4);

    println!(
        "{:<path_width$}  {:>12}  LAST MODIFIED",
        "PATH", "SIZE",
        path_width = path_width,
    );
    println!("{}", "-".repeat(path_width + 2 + 12 + 2 + 24));

    for file in &all_files {
        println!(
            "{:<path_width$}  {:>12}  {}",
            file.path,
            format_size(file.size_bytes),
            file.last_modified,
            path_width = path_width,
        );
    }

    println!("\n{} file(s)", all_files.len());

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
