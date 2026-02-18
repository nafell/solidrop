use axum::{extract::Query, extract::State, routing::get, Json, Router};
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
    next_token: Option<String>,
}

#[derive(Serialize)]
struct FileEntry {
    key: String,
    size: i64,
    last_modified: Option<String>,
    content_hash: Option<String>,
}

#[derive(Serialize)]
struct ListResponse {
    files: Vec<FileEntry>,
    next_token: Option<String>,
}

async fn list_files(
    State(state): State<AppState>,
    Query(params): Query<ListParams>,
) -> Result<Json<ListResponse>, AppError> {
    let limit = params.limit.unwrap_or(100).clamp(1, 100);

    let mut req = state
        .s3
        .list_objects_v2()
        .bucket(&state.config.s3_bucket)
        .max_keys(limit);

    if let Some(ref prefix) = params.prefix {
        req = req.prefix(prefix);
    }

    if let Some(ref token) = params.next_token {
        req = req.continuation_token(token);
    }

    let output = req
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let mut files = Vec::new();

    for obj in output.contents() {
        let key = match obj.key() {
            Some(k) => k.to_string(),
            None => continue,
        };

        let size = obj.size().unwrap_or(0);

        let last_modified: Option<String> = obj.last_modified().and_then(|dt| {
            dt.fmt(aws_sdk_s3::primitives::DateTimeFormat::DateTime)
                .ok()
        });

        let content_hash = match state
            .s3
            .head_object()
            .bucket(&state.config.s3_bucket)
            .key(&key)
            .send()
            .await
        {
            Ok(head) => head.metadata().and_then(|m| m.get("content-hash").cloned()),
            Err(_) => None,
        };

        files.push(FileEntry {
            key,
            size,
            last_modified,
            content_hash,
        });
    }

    let next_token = output.next_continuation_token().map(|s| s.to_string());

    Ok(Json(ListResponse { files, next_token }))
}
