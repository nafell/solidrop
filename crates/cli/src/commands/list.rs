use anyhow::Result;

use crate::api_client::ApiClient;

fn format_size(bytes: i64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else if bytes < 1024 * 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    }
}

pub async fn run(api: &ApiClient, prefix: Option<&str>) -> Result<()> {
    let mut all_files = Vec::new();
    let mut next_token: Option<String> = None;

    loop {
        let (files, token) = api
            .list_files(prefix, Some(100), next_token.as_deref())
            .await?;
        all_files.extend(files);
        next_token = token;
        if next_token.is_none() {
            break;
        }
    }

    for file in &all_files {
        let size = format_size(file.size);
        let modified = file.last_modified.as_deref().unwrap_or("\u{2014}");
        println!("{:>10}  {}  {}", size, modified, file.key);
    }

    println!("\n{} file(s)", all_files.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_bytes() {
        assert_eq!(format_size(0), "0 B");
        assert_eq!(format_size(512), "512 B");
        assert_eq!(format_size(1023), "1023 B");
    }

    #[test]
    fn test_format_size_kilobytes() {
        assert_eq!(format_size(1024), "1.0 KB");
        assert_eq!(format_size(5325), "5.2 KB");
        assert_eq!(format_size(1024 * 1024 - 1), "1024.0 KB");
    }

    #[test]
    fn test_format_size_megabytes() {
        assert_eq!(format_size(1024 * 1024), "1.0 MB");
        assert_eq!(format_size(30 * 1024 * 1024), "30.0 MB");
    }

    #[test]
    fn test_format_size_gigabytes() {
        assert_eq!(format_size(1024 * 1024 * 1024), "1.0 GB");
        assert_eq!(format_size(2 * 1024 * 1024 * 1024_i64), "2.0 GB");
    }
}
