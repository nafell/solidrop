use axum::{extract::State, http::StatusCode, routing::get, Json, Router};
use serde::Serialize;

use super::AppState;

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/files", get(list_files))
}

#[derive(Serialize)]
struct NotImplementedResponse {
    error: &'static str,
}

async fn list_files(State(state): State<AppState>) -> (StatusCode, Json<NotImplementedResponse>) {
    let _configured_bucket = &state.config.s3_bucket;
    let _s3_client = &state.s3;

    // TODO: Implement S3 ListObjects-based file listing
    (
        StatusCode::NOT_IMPLEMENTED,
        Json(NotImplementedResponse {
            error: "file listing is not implemented yet",
        }),
    )
}
