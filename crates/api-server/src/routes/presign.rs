use std::time::Duration;

use aws_sdk_s3::presigning::PresigningConfig;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use crate::error::AppError;

use super::AppState;

const PRESIGN_EXPIRY_SECS: u64 = 3600;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/presign/upload", post(presign_upload))
        .route("/api/v1/presign/download", post(presign_download))
}

#[derive(Deserialize)]
struct UploadRequest {
    path: String,
    /// SHA-256 hex digest of the plaintext content (before encryption).
    /// Embedded as S3 object metadata for end-to-end integrity verification.
    content_hash: String,
    /// Expected upload size in bytes. Accepted for future quota enforcement.
    size_bytes: u64,
}

#[derive(Deserialize)]
struct DownloadRequest {
    path: String,
}

#[derive(Serialize)]
struct PresignUploadResponse {
    url: String,
    expires_in: u64,
}

#[derive(Serialize)]
struct PresignDownloadResponse {
    url: String,
    expires_in: u64,
    content_hash: Option<String>,
}

fn build_presigning_config() -> Result<PresigningConfig, AppError> {
    PresigningConfig::builder()
        .expires_in(Duration::from_secs(PRESIGN_EXPIRY_SECS))
        .build()
        .map_err(|e| AppError::Internal(e.to_string()))
}

async fn presign_upload(
    State(state): State<AppState>,
    Json(body): Json<UploadRequest>,
) -> Result<Json<PresignUploadResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }
    if body.content_hash.is_empty() {
        return Err(AppError::BadRequest(
            "content_hash must not be empty".into(),
        ));
    }
    // size_bytes accepted for future quota enforcement
    let _ = body.size_bytes;

    // Use s3_presign (configured with the public endpoint) so the signed `host`
    // header in the URL matches the endpoint clients will connect to.
    // metadata("content-hash", ...) embeds the hash as a signed header constraint;
    // S3 will reject PUTs that omit or mismatch x-amz-meta-content-hash.
    let presigned = state
        .s3_presign
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .metadata("content-hash", &body.content_hash)
        .presigned(build_presigning_config()?)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(PresignUploadResponse {
        url: presigned.uri().to_string(),
        expires_in: PRESIGN_EXPIRY_SECS,
    }))
}

async fn presign_download(
    State(state): State<AppState>,
    Json(body): Json<DownloadRequest>,
) -> Result<Json<PresignDownloadResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }

    // HeadObject retrieves the stored content-hash metadata for integrity verification.
    // AWS normalises all user-defined metadata keys to lowercase, so "content-hash"
    // is the canonical form for both storage and retrieval.
    let head = state
        .s3
        .head_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let content_hash = head
        .metadata()
        .and_then(|m| m.get("content-hash"))
        .map(String::from);

    let presigned = state
        .s3_presign
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .presigned(build_presigning_config()?)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(PresignDownloadResponse {
        url: presigned.uri().to_string(),
        expires_in: PRESIGN_EXPIRY_SECS,
        content_hash,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::SdkConfig;
    use axum_test::TestServer;
    use serde_json::json;

    fn make_test_state() -> AppState {
        use crate::config::AppConfig;
        let config = AppConfig {
            port: 3000,
            s3_bucket: "test-bucket".into(),
            api_key: "test-key".into(),
            aws_region: "us-east-1".into(),
            s3_endpoint_url: None,
            s3_force_path_style: false,
            s3_public_endpoint_url: None,
        };
        // Fake S3 clients — S3 is never reached in validation tests.
        let sdk_config = SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        let s3 = aws_sdk_s3::Client::new(&sdk_config);
        let s3_presign = aws_sdk_s3::Client::new(&sdk_config);
        AppState {
            s3,
            s3_presign,
            config,
        }
    }

    #[tokio::test]
    async fn upload_empty_path_returns_400() {
        let app = router().with_state(make_test_state());
        let server = TestServer::new(app).unwrap();

        let resp = server
            .post("/api/v1/presign/upload")
            .json(&json!({ "path": "", "content_hash": "abc123", "size_bytes": 100 }))
            .await;
        assert_eq!(resp.status_code(), 400);
    }

    #[tokio::test]
    async fn upload_empty_content_hash_returns_400() {
        let app = router().with_state(make_test_state());
        let server = TestServer::new(app).unwrap();

        let resp = server
            .post("/api/v1/presign/upload")
            .json(&json!({ "path": "test.clip.enc", "content_hash": "", "size_bytes": 100 }))
            .await;
        assert_eq!(resp.status_code(), 400);
    }

    #[tokio::test]
    async fn download_empty_path_returns_400() {
        let app = router().with_state(make_test_state());
        let server = TestServer::new(app).unwrap();

        let resp = server
            .post("/api/v1/presign/download")
            .json(&json!({ "path": "" }))
            .await;
        assert_eq!(resp.status_code(), 400);
    }
}
