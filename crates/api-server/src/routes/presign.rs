use axum::{extract::State, http::StatusCode, routing::post, Json, Router};
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
struct NotImplementedResponse {
    error: &'static str,
}

async fn presign_upload(
    State(state): State<AppState>,
    Json(body): Json<UploadRequest>,
) -> (StatusCode, Json<NotImplementedResponse>) {
    let _request_shape = (&body.path, &body.content_hash, body.size_bytes);
    let _configured_bucket = &state.config.s3_bucket;
    let _s3_client = &state.s3;

    // TODO: Implement presigned upload URL generation
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(NotImplementedResponse {
            error: "presigned upload URL generation is not implemented yet",
        }),
    )
}

#[derive(Deserialize)]
struct DownloadRequest {
    path: String,
}

async fn presign_download(
    State(state): State<AppState>,
    Json(body): Json<DownloadRequest>,
) -> (StatusCode, Json<NotImplementedResponse>) {
    let _requested_path = &body.path;
    let _configured_bucket = &state.config.s3_bucket;
    let _s3_client = &state.s3;

    // TODO: Implement presigned download URL generation
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(NotImplementedResponse {
            error: "presigned download URL generation is not implemented yet",
        }),
    )
}
