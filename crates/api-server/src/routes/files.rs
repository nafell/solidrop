use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use super::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files", get(list_files))
}

#[derive(Serialize)]
struct FileEntry {
    path: String,
    size_bytes: u64,
    last_modified: String,
    content_hash: String,
}

#[derive(Serialize)]
struct ListFilesResponse {
    files: Vec<FileEntry>,
    next_token: Option<String>,
}

async fn list_files(State(_state): State<AppState>) -> Json<ListFilesResponse> {
    // TODO: Implement S3 ListObjects-based file listing
    Json(ListFilesResponse {
        files: vec![],
        next_token: None,
    })
}
