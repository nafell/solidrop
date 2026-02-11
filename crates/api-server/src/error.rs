use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("unauthorized")]
    Unauthorized,

    #[error("not found: {0}")]
    NotFound(String),

    #[error("bad request: {0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "FILE_NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Internal(msg) => {
                tracing::error!("internal error: {msg}");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred.".into(),
                )
            }
        };

        let body = json!({
            "error": {
                "code": code,
                "message": message,
            }
        });

        (status, Json(body)).into_response()
    }
}
