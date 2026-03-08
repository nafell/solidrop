use axum::{extract::State, routing::post, Json, Router};
use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
use serde::Deserialize;
use serde_json::{json, Value};

use super::AppState;
use crate::error::AppError;

/// Characters to percent-encode in S3 copy_source keys.
/// Per RFC 3986, unreserved characters (ALPHA, DIGIT, '-', '.', '_', '~') are
/// left as-is. '/' is also preserved since it serves as S3's path delimiter.
const S3_KEY_ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'/')
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

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
    let encoded_from = utf8_percent_encode(&body.from, S3_KEY_ENCODE_SET);

    // Copy to new location
    state
        .s3
        .copy_object()
        .bucket(bucket)
        .copy_source(format!("{bucket}/{encoded_from}"))
        .key(&body.to)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.to_string()))?;

    // Delete the original (best-effort: copy already succeeded at this point)
    if let Err(e) = state
        .s3
        .delete_object()
        .bucket(bucket)
        .key(&body.from)
        .send()
        .await
    {
        tracing::error!(
            from = %body.from,
            to = %body.to,
            error = %e,
            "move: delete of original failed after successful copy — \
             object now exists at both source and destination"
        );
        return Err(AppError::Internal(e.to_string()));
    }

    Ok(Json(json!({"moved": true})))
}
