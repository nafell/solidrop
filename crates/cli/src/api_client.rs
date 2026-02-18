use anyhow::{bail, Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::CliConfig;

pub struct ApiClient {
    client: Client,
    base_url: String,
    api_key: String,
}

// --- Request/Response types matching the API server ---

#[derive(Serialize)]
struct PresignUploadRequest {
    path: String,
    content_hash: String,
    size_bytes: u64,
}

#[derive(Deserialize)]
struct PresignUploadResponse {
    upload_url: String,
}

#[derive(Serialize)]
struct PresignDownloadRequest {
    path: String,
}

#[derive(Deserialize)]
struct PresignDownloadResponse {
    download_url: String,
}

#[derive(Debug, Deserialize)]
pub struct FileEntry {
    pub key: String,
    pub size: i64,
    pub last_modified: Option<String>,
    pub content_hash: Option<String>,
}

#[derive(Deserialize)]
struct ListResponse {
    pub files: Vec<FileEntry>,
    pub next_token: Option<String>,
}

#[derive(Serialize)]
struct MoveRequest {
    from: String,
    to: String,
}

#[derive(Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Deserialize)]
struct ApiErrorDetail {
    code: String,
    message: String,
}

impl ApiClient {
    pub fn from_config(config: &CliConfig) -> Result<Self> {
        let api_key = std::env::var(&config.server.api_key_env).with_context(|| {
            format!(
                "environment variable '{}' not set (required for API authentication)",
                config.server.api_key_env
            )
        })?;

        let client = Client::new();
        let base_url = config.server.endpoint.trim_end_matches('/').to_string();

        Ok(Self {
            client,
            base_url,
            api_key,
        })
    }

    /// POST /presign/upload — returns a presigned S3 upload URL.
    pub async fn presign_upload(
        &self,
        path: &str,
        content_hash: &str,
        size_bytes: u64,
    ) -> Result<String> {
        let body = PresignUploadRequest {
            path: path.to_string(),
            content_hash: content_hash.to_string(),
            size_bytes,
        };
        let resp = self
            .client
            .post(format!("{}/presign/upload", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("failed to request presigned upload URL")?;

        let resp = Self::check_response(resp).await?;
        let parsed: PresignUploadResponse = resp
            .json()
            .await
            .context("failed to parse presign upload response")?;
        Ok(parsed.upload_url)
    }

    /// POST /presign/download — returns a presigned S3 download URL.
    pub async fn presign_download(&self, path: &str) -> Result<String> {
        let body = PresignDownloadRequest {
            path: path.to_string(),
        };
        let resp = self
            .client
            .post(format!("{}/presign/download", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("failed to request presigned download URL")?;

        let resp = Self::check_response(resp).await?;
        let parsed: PresignDownloadResponse = resp
            .json()
            .await
            .context("failed to parse presign download response")?;
        Ok(parsed.download_url)
    }

    /// GET /files — list files with optional prefix and pagination.
    pub async fn list_files(
        &self,
        prefix: Option<&str>,
        limit: Option<i32>,
        next_token: Option<&str>,
    ) -> Result<(Vec<FileEntry>, Option<String>)> {
        let mut req = self
            .client
            .get(format!("{}/files", self.base_url))
            .bearer_auth(&self.api_key);

        if let Some(p) = prefix {
            req = req.query(&[("prefix", p)]);
        }
        if let Some(l) = limit {
            req = req.query(&[("limit", l.to_string().as_str())]);
        }
        if let Some(t) = next_token {
            req = req.query(&[("next_token", t)]);
        }

        let resp = req.send().await.context("failed to list files")?;
        let resp = Self::check_response(resp).await?;
        let parsed: ListResponse = resp.json().await.context("failed to parse list response")?;
        Ok((parsed.files, parsed.next_token))
    }

    /// DELETE /files/{path} — delete a remote file.
    pub async fn delete_file(&self, path: &str) -> Result<()> {
        let resp = self
            .client
            .delete(format!("{}/files/{}", self.base_url, path))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .context("failed to delete file")?;

        Self::check_response(resp).await?;
        Ok(())
    }

    /// POST /files/move — move (rename) a remote file.
    pub async fn move_file(&self, from: &str, to: &str) -> Result<()> {
        let body = MoveRequest {
            from: from.to_string(),
            to: to.to_string(),
        };
        let resp = self
            .client
            .post(format!("{}/files/move", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
            .context("failed to move file")?;

        Self::check_response(resp).await?;
        Ok(())
    }

    /// PUT encrypted bytes directly to S3 via presigned URL (no auth header needed).
    pub async fn put_to_s3(&self, presigned_url: &str, data: &[u8]) -> Result<()> {
        let resp = self
            .client
            .put(presigned_url)
            .header("Content-Type", "application/octet-stream")
            .body(data.to_vec())
            .send()
            .await
            .context("failed to upload to S3")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("S3 upload failed (HTTP {}): {}", status, body);
        }
        Ok(())
    }

    /// GET encrypted bytes from S3 via presigned URL (no auth header needed).
    pub async fn get_from_s3(&self, presigned_url: &str) -> Result<Vec<u8>> {
        let resp = self
            .client
            .get(presigned_url)
            .send()
            .await
            .context("failed to download from S3")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            bail!("S3 download failed (HTTP {}): {}", status, body);
        }

        let bytes = resp
            .bytes()
            .await
            .context("failed to read S3 response body")?;
        Ok(bytes.to_vec())
    }

    /// Check HTTP response status; extract API error body if present.
    async fn check_response(resp: reqwest::Response) -> Result<reqwest::Response> {
        if resp.status().is_success() {
            return Ok(resp);
        }

        let status = resp.status();
        let body_text = resp.text().await.unwrap_or_default();

        // Try to parse the API's structured error format
        if let Ok(api_err) = serde_json::from_str::<ApiErrorBody>(&body_text) {
            bail!(
                "API error (HTTP {}): [{}] {}",
                status,
                api_err.error.code,
                api_err.error.message
            );
        }

        bail!("API error (HTTP {}): {}", status, body_text);
    }
}
