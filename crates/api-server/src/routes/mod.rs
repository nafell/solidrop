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

#[cfg(test)]
mod tests {
    use super::*;
    use aws_config::SdkConfig;
    use axum::{middleware::from_fn_with_state, routing::get, Router};
    use axum_test::TestServer;

    fn make_test_state(api_key: &str) -> AppState {
        use crate::config::AppConfig;
        let config = AppConfig {
            port: 3000,
            s3_bucket: "test".into(),
            api_key: api_key.to_string(),
            aws_region: "us-east-1".into(),
            s3_endpoint_url: None,
            s3_force_path_style: false,
            s3_public_endpoint_url: None,
        };
        let sdk_config = SdkConfig::builder()
            .behavior_version(aws_config::BehaviorVersion::latest())
            .build();
        let s3 = aws_sdk_s3::Client::new(&sdk_config);
        let s3_presign = aws_sdk_s3::Client::new(&sdk_config);
        AppState {
            s3,
            s3_presign,
            config,
        }
    }

    fn make_auth_router(api_key: &str) -> Router {
        let state = make_test_state(api_key);
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .route_layer(from_fn_with_state(state.clone(), auth_middleware))
            .with_state(state)
    }

    #[tokio::test]
    async fn no_auth_header_returns_401() {
        let server = TestServer::new(make_auth_router("secret")).unwrap();
        let resp = server.get("/test").await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn wrong_token_returns_401() {
        let server = TestServer::new(make_auth_router("secret")).unwrap();
        let resp = server
            .get("/test")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_static("Bearer wrong"),
            )
            .await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn correct_token_passes_through() {
        let server = TestServer::new(make_auth_router("secret")).unwrap();
        let resp = server
            .get("/test")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_static("Bearer secret"),
            )
            .await;
        assert_eq!(resp.status_code(), 200);
    }
}
