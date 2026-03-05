use anyhow::{Context, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub struct SolidropApi {
    base_url: String,
    api_key: String,
    client: Client,
}

#[derive(Serialize)]
struct PresignUploadRequest<'a> {
    path: &'a str,
    content_hash: &'a str,
    size_bytes: u64,
}

#[derive(Serialize)]
struct PresignDownloadRequest<'a> {
    path: &'a str,
}

#[derive(Deserialize)]
pub struct PresignResponse {
    pub url: String,
}

#[derive(Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub size_bytes: u64,
    pub last_modified: String,
    pub content_hash: String,
}

#[derive(Deserialize)]
pub struct FilesResponse {
    pub files: Vec<FileEntry>,
    pub next_token: Option<String>,
}

impl SolidropApi {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: Client::new(),
        }
    }

    pub async fn presign_upload(
        &self,
        path: &str,
        content_hash: &str,
        size_bytes: u64,
    ) -> Result<PresignResponse> {
        let url = format!("{}/api/v1/presign/upload", self.base_url);
        self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&PresignUploadRequest {
                path,
                content_hash,
                size_bytes,
            })
            .send()
            .await
            .context("presign/upload request failed")?
            .error_for_status()
            .context("presign/upload returned error status")?
            .json::<PresignResponse>()
            .await
            .context("failed to parse presign/upload response")
    }

    pub async fn presign_download(&self, path: &str) -> Result<PresignResponse> {
        let url = format!("{}/api/v1/presign/download", self.base_url);
        self.client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&PresignDownloadRequest { path })
            .send()
            .await
            .context("presign/download request failed")?
            .error_for_status()
            .context("presign/download returned error status")?
            .json::<PresignResponse>()
            .await
            .context("failed to parse presign/download response")
    }

    pub async fn list_files(&self, prefix: Option<&str>) -> Result<FilesResponse> {
        let url = format!("{}/api/v1/files", self.base_url);
        let mut req = self.client.get(&url).bearer_auth(&self.api_key);
        if let Some(p) = prefix {
            req = req.query(&[("prefix", p)]);
        }
        req.send()
            .await
            .context("list files request failed")?
            .error_for_status()
            .context("/api/v1/files returned error status")?
            .json::<FilesResponse>()
            .await
            .context("failed to parse files response")
    }

    /// Upload bytes directly to a presigned PUT URL.
    pub async fn put_object(&self, presigned_url: &str, data: Vec<u8>) -> Result<()> {
        self.client
            .put(presigned_url)
            .body(data)
            .send()
            .await
            .context("PUT to presigned URL failed")?
            .error_for_status()
            .context("PUT to presigned URL returned error status")?;
        Ok(())
    }

    /// Download bytes from a presigned GET URL.
    pub async fn get_object(&self, presigned_url: &str) -> Result<Vec<u8>> {
        let bytes = self
            .client
            .get(presigned_url)
            .send()
            .await
            .context("GET from presigned URL failed")?
            .error_for_status()
            .context("GET from presigned URL returned error status")?
            .bytes()
            .await
            .context("failed to read response bytes")?;
        Ok(bytes.to_vec())
    }
}
