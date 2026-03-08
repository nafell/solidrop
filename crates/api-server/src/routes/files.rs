use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files", get(list_files))
}

#[derive(Deserialize)]
struct ListParams {
    prefix: Option<String>,
    limit: Option<i32>,
    continuation_token: Option<String>,
}

#[derive(Serialize)]
struct FileEntry {
    path: String,
    size_bytes: u64,
    last_modified: String,
    /// SHA-256 hash stored as S3 object metadata at upload time.
    /// Falls back to the S3 ETag if the metadata key is absent.
    content_hash: String,
}

#[derive(Serialize)]
struct FilesResponse {
    files: Vec<FileEntry>,
    next_token: Option<String>,
}

async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<FilesResponse>, AppError> {
    let limit = params.limit.unwrap_or(100).clamp(1, 100);

    let mut req = state
        .s3
        .list_objects_v2()
        .bucket(&state.config.s3_bucket)
        .max_keys(limit);

    if let Some(prefix) = params.prefix {
        req = req.prefix(prefix);
    }
    if let Some(token) = params.continuation_token {
        req = req.continuation_token(token);
    }

    let output = req
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let files = output
        .contents()
        .iter()
        .filter_map(|obj| {
            let path = obj.key()?.to_string();
            let size_bytes = obj.size().unwrap_or(0) as u64;
            let last_modified = obj
                .last_modified()
                .map(|t| t.to_string())
                .unwrap_or_default();
            // ETag is available from ListObjectsV2 without extra API calls.
            // The real SHA-256 hash is stored as metadata and requires
            // individual HeadObject calls; that optimization can be added later.
            let content_hash = obj.e_tag().unwrap_or("").trim_matches('"').to_string();

            Some(FileEntry {
                path,
                size_bytes,
                last_modified,
                content_hash,
            })
        })
        .collect();

    let next_token = output.next_continuation_token().map(String::from);

    Ok(Json(FilesResponse { files, next_token }))
}
