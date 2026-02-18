use aws_sdk_s3::presigning::PresigningConfig;
use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/presign/upload", post(presign_upload))
        .route("/api/v1/presign/download", post(presign_download))
}

#[derive(Deserialize)]
struct UploadRequest {
    path: String,
    content_hash: String,
    size_bytes: u64,
}

#[derive(Serialize)]
struct UploadResponse {
    upload_url: String,
}

#[derive(Deserialize)]
struct DownloadRequest {
    path: String,
}

#[derive(Serialize)]
struct DownloadResponse {
    download_url: String,
}

async fn presign_upload(
    State(state): State<AppState>,
    Json(body): Json<UploadRequest>,
) -> Result<Json<UploadResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }

    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let presigned = state
        .s3
        .put_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .metadata("content-hash", &body.content_hash)
        .metadata("original-size", body.size_bytes.to_string())
        .presigned(presigning_config)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let url = maybe_rewrite_url(presigned.uri().to_string(), &state);

    Ok(Json(UploadResponse { upload_url: url }))
}

async fn presign_download(
    State(state): State<AppState>,
    Json(body): Json<DownloadRequest>,
) -> Result<Json<DownloadResponse>, AppError> {
    if body.path.is_empty() {
        return Err(AppError::BadRequest("path must not be empty".into()));
    }

    let presigning_config = PresigningConfig::expires_in(Duration::from_secs(3600))
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let presigned = state
        .s3
        .get_object()
        .bucket(&state.config.s3_bucket)
        .key(&body.path)
        .presigned(presigning_config)
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let url = maybe_rewrite_url(presigned.uri().to_string(), &state);

    Ok(Json(DownloadResponse { download_url: url }))
}

/// If both `s3_public_endpoint_url` and `s3_endpoint_url` are configured,
/// rewrite the presigned URL so it is accessible from outside the Docker network.
fn maybe_rewrite_url(url: String, state: &AppState) -> String {
    if let (Some(public_endpoint), Some(internal_endpoint)) = (
        &state.config.s3_public_endpoint_url,
        &state.config.s3_endpoint_url,
    ) {
        crate::s3_client::rewrite_presigned_url_for_public_access(
            &url,
            internal_endpoint,
            public_endpoint,
        )
    } else {
        url
    }
}
