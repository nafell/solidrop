use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::AppState;

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
struct PresignResponse {
    url: String,
    expires_in: u64,
}

async fn presign_upload(
    State(_state): State<AppState>,
    Json(_body): Json<UploadRequest>,
) -> Json<PresignResponse> {
    // TODO: Implement presigned upload URL generation
    Json(PresignResponse {
        url: String::new(),
        expires_in: 3600,
    })
}

#[derive(Deserialize)]
struct DownloadRequest {
    path: String,
}

async fn presign_download(
    State(_state): State<AppState>,
    Json(_body): Json<DownloadRequest>,
) -> Json<PresignResponse> {
    // TODO: Implement presigned download URL generation
    Json(PresignResponse {
        url: String::new(),
        expires_in: 3600,
    })
}
