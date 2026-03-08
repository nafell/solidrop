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
pub struct PresignUploadResponse {
    pub url: String,
}

#[derive(Deserialize)]
pub struct PresignDownloadResponse {
    pub url: String,
    pub content_hash: Option<String>,
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
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            client: Client::new(),
        }
    }

    pub async fn presign_upload(
        &self,
        path: &str,
        content_hash: &str,
        size_bytes: u64,
    ) -> Result<PresignUploadResponse> {
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
            .json::<PresignUploadResponse>()
            .await
            .context("failed to parse presign/upload response")
    }

    pub async fn presign_download(&self, path: &str) -> Result<PresignDownloadResponse> {
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
            .json::<PresignDownloadResponse>()
            .await
            .context("failed to parse presign/download response")
    }

    pub async fn list_files(
        &self,
        prefix: Option<&str>,
        continuation_token: Option<&str>,
    ) -> Result<FilesResponse> {
        let url = format!("{}/api/v1/files", self.base_url);
        let mut req = self.client.get(&url).bearer_auth(&self.api_key);
        if let Some(p) = prefix {
            req = req.query(&[("prefix", p)]);
        }
        if let Some(token) = continuation_token {
            req = req.query(&[("continuation_token", token)]);
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
    /// `content_hash` is sent as `x-amz-meta-content-hash` to satisfy the
    /// signed-header constraint embedded in the presigned URL.
    pub async fn put_object(
        &self,
        presigned_url: &str,
        data: Vec<u8>,
        content_hash: Option<&str>,
    ) -> Result<()> {
        let mut req = self.client.put(presigned_url).body(data);
        if let Some(hash) = content_hash {
            req = req.header("x-amz-meta-content-hash", hash);
        }
        req.send()
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

#[cfg(test)]
mod tests {
    use super::*;
    use httpmock::prelude::*;

    #[test]
    fn base_url_trailing_slash_is_trimmed() {
        let api = SolidropApi::new("http://localhost:3000/".to_string(), "key".to_string());
        assert_eq!(api.base_url, "http://localhost:3000");
    }

    #[test]
    fn base_url_without_slash_unchanged() {
        let api = SolidropApi::new("http://localhost:3000".to_string(), "key".to_string());
        assert_eq!(api.base_url, "http://localhost:3000");
    }

    /// Verifies that `list_files` passes `continuation_token` as a query parameter.
    #[tokio::test]
    async fn list_files_sends_continuation_token_as_query_param() {
        let server = MockServer::start_async().await;

        server.mock_async(|when, then| {
            when.method(GET)
                .path("/api/v1/files")
                .query_param("continuation_token", "tok1");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"files":[{"path":"b.clip.enc","size_bytes":200,"last_modified":"2024-01-02T00:00:00Z","content_hash":"bbb"}],"next_token":null}"#);
        }).await;

        let api = SolidropApi::new(server.base_url(), "anykey".to_string());
        let resp = api.list_files(None, Some("tok1")).await.unwrap();

        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.files[0].path, "b.clip.enc");
        assert!(resp.next_token.is_none());
    }

    /// Verifies that `list_files` without a continuation token omits the query param
    /// (mock requires the param, so a mismatch would return a 404).
    #[tokio::test]
    async fn list_files_first_page_has_no_token() {
        let server = MockServer::start_async().await;

        server.mock_async(|when, then| {
            when.method(GET).path("/api/v1/files");
            then.status(200)
                .header("content-type", "application/json")
                .body(r#"{"files":[{"path":"a.clip.enc","size_bytes":100,"last_modified":"2024-01-01T00:00:00Z","content_hash":"aaa"}],"next_token":"tok1"}"#);
        }).await;

        let api = SolidropApi::new(server.base_url(), "anykey".to_string());
        let resp = api.list_files(None, None).await.unwrap();

        assert_eq!(resp.files.len(), 1);
        assert_eq!(resp.next_token.as_deref(), Some("tok1"));
    }
}
