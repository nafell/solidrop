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
    /// SHA-256 hash of the plaintext content (before encryption).
    /// Accepted for future quota / deduplication use; not embedded in the URL.
    content_hash: String,
    /// Expected upload size in bytes. Accepted for future quota enforcement.
    size_bytes: u64,
}

#[derive(Deserialize)]
struct DownloadRequest {
    path: String,
}

#[derive(Serialize)]
struct PresignResponse {
    url: String,
    expires_in: u64,
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
) -> Result<Json<PresignResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }
    // Suppress unused-field warnings until quota logic is added.
    let _ = (&body.content_hash, body.size_bytes);

    // Use s3_presign (configured with the public endpoint) so the signed `host`
    // header in the URL matches the endpoint clients will connect to.
    let presigned = state
        .s3_presign
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .presigned(build_presigning_config()?)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(PresignResponse {
        url: presigned.uri().to_string(),
        expires_in: PRESIGN_EXPIRY_SECS,
    }))
}

async fn presign_download(
    State(state): State<AppState>,
    Json(body): Json<DownloadRequest>,
) -> Result<Json<PresignResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }

    let presigned = state
        .s3_presign
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .presigned(build_presigning_config()?)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(PresignResponse {
        url: presigned.uri().to_string(),
        expires_in: PRESIGN_EXPIRY_SECS,
    }))
}
