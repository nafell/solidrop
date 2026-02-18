use axum::{extract::Path, extract::State, routing::delete, Json, Router};
use serde_json::{json, Value};

use super::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files/*path", delete(delete_file))
}

async fn delete_file(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<Json<Value>, AppError> {
    // Verify the object exists
    state
        .s3
        .head_object()
        .bucket(&state.config.s3_bucket)
        .key(&path)
        .send()
        .await
        .map_err(|_| AppError::NotFound(format!("file not found: {path}")))?;

    // Delete the object
    state
        .s3
        .delete_object()
        .bucket(&state.config.s3_bucket)
        .key(&path)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(Json(json!({"deleted": true})))
}
