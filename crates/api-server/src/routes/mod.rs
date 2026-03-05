use aws_sdk_s3::Client;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Router,
};

use crate::{config::AppConfig, error::AppError};

mod files;
mod health;
mod presign;

#[derive(Clone)]
pub struct AppState {
    /// Client for regular S3 API calls (ListObjects, HeadObject, etc.).
    /// Configured with the internal Docker endpoint in local dev.
    pub s3: Client,
    /// Client used only for generating presigned URLs.
    /// Configured with the public endpoint so that the signed `host` header
    /// matches the URL clients will actually connect to.
    pub s3_presign: Client,
    pub config: AppConfig,
}

/// Routes that do not require authentication.
pub fn health_router() -> Router<AppState> {
    health::router()
}

/// Routes that require Bearer token authentication.
/// Apply `auth_middleware` to this router in main.rs after the state is available.
pub fn api_router() -> Router<AppState> {
    Router::new()
        .merge(presign::router())
        .merge(files::router())
}

/// Bearer token authentication middleware.
///
/// Checks `Authorization: Bearer <token>` against `config.api_key`.
/// Must be applied via `route_layer(from_fn_with_state(...))` in main.rs.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    if token != state.config.api_key {
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
