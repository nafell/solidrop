use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};

use super::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files/move", post(move_file))
}

#[derive(Deserialize)]
struct MoveRequest {
    from: String,
    to: String,
}

async fn move_file(
    State(state): State<AppState>,
    Json(body): Json<MoveRequest>,
) -> Result<Json<Value>, AppError> {
    if body.from.is_empty() {
        return Err(AppError::BadRequest("'from' must not be empty".into()));
    }
    if body.to.is_empty() {
        return Err(AppError::BadRequest("'to' must not be empty".into()));
    }

    let bucket = &state.config.s3_bucket;

    // Copy to new location
    state
        .s3
        .copy_object()
        .bucket(bucket)
        .copy_source(format!("{bucket}/{}", &body.from))
        .key(&body.to)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Delete the original
    state
        .s3
        .delete_object()
        .bucket(bucket)
        .key(&body.from)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({"moved": true})))
}
