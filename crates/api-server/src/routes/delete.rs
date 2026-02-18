use aws_sdk_s3::error::SdkError;
use axum::{extract::Path, extract::State, routing::delete, Json, Router};
use serde_json::{json, Value};

use super::AppState;
use crate::error::AppError;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files/*path", delete(delete_file))
}

/// Check if an S3 SDK error is a 404 (object not found).
/// Only `ServiceError` with HTTP 404 qualifies; timeouts, auth failures, etc. do not.
fn is_not_found<E>(err: &SdkError<E>) -> bool {
    matches!(err, SdkError::ServiceError(e) if e.raw().status().as_u16() == 404)
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
        .map_err(|e| {
            if is_not_found(&e) {
                AppError::NotFound(format!("file not found: {path}"))
            } else {
                AppError::Internal(format!("S3 head_object failed for {path}: {e}"))
            }
        })?;

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
