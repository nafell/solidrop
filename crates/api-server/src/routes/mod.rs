use aws_sdk_s3::Client;
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
    Router,
};

use crate::{config::AppConfig, error::AppError};

pub mod cache;
pub mod delete;
pub mod file_move;
pub mod files;
mod health;
pub mod presign;

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
        .merge(delete::router())
        .merge(file_move::router())
        .merge(cache::router())
}

/// Bearer token authentication middleware.
///
/// The client sends `Authorization: Bearer <api_token_hex>` where `api_token`
/// is derived from the master key via HKDF (see ADR-003).
///
/// The server holds only `SOLIDROP_API_KEY_VERIFIER_SHA256 = hex(SHA-256(api_token))`.
/// Verification: `SHA-256(received_token_bytes)` must equal the stored verifier.
/// Comparison is performed in constant time to resist timing attacks.
pub async fn auth_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token_hex = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    // Decode the received hex token to raw bytes.
    let token_bytes = hex::decode(token_hex).map_err(|_| AppError::Unauthorized)?;
    if token_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }

    // Compute SHA-256 of the received token bytes.
    let computed_hash = solidrop_crypto::hash::sha256_raw(&token_bytes);

    // Decode the stored verifier to bytes for constant-time comparison.
    let stored_bytes =
        hex::decode(&state.config.api_key_verifier).map_err(|_| AppError::Unauthorized)?;
    if stored_bytes.len() != 32 {
        return Err(AppError::Unauthorized);
    }

    use subtle::ConstantTimeEq;
    if computed_hash.ct_eq(stored_bytes.as_slice()).unwrap_u8() != 1 {
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

    /// 32 zero bytes, hex-encoded. Used as a fixed test API token.
    const TEST_TOKEN_HEX: &str = "0000000000000000000000000000000000000000000000000000000000000000";

    fn verifier_for(token_hex: &str) -> String {
        let token_bytes = hex::decode(token_hex).unwrap();
        hex::encode(solidrop_crypto::hash::sha256_raw(&token_bytes))
    }

    fn make_test_state(verifier_hex: &str) -> AppState {
        let config = AppConfig {
            port: 3000,
            s3_bucket: "test".into(),
            api_key_verifier: verifier_hex.to_string(),
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

    fn make_auth_router() -> Router {
        let verifier = verifier_for(TEST_TOKEN_HEX);
        let state = make_test_state(&verifier);
        Router::new()
            .route("/test", get(|| async { "ok" }))
            .route_layer(from_fn_with_state(state.clone(), auth_middleware))
            .with_state(state)
    }

    #[tokio::test]
    async fn no_auth_header_returns_401() {
        let server = TestServer::new(make_auth_router()).unwrap();
        let resp = server.get("/test").await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn wrong_token_returns_401() {
        let server = TestServer::new(make_auth_router()).unwrap();
        let wrong_token = "ff".repeat(32); // correct length but wrong value
        let resp = server
            .get("/test")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_str(&format!("Bearer {wrong_token}")).unwrap(),
            )
            .await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn non_hex_token_returns_401() {
        let server = TestServer::new(make_auth_router()).unwrap();
        let resp = server
            .get("/test")
            .add_header(
                axum::http::header::AUTHORIZATION,
                axum::http::HeaderValue::from_static("Bearer not-valid-hex"),
            )
            .await;
        assert_eq!(resp.status_code(), 401);
    }

    #[tokio::test]
    async fn correct_token_passes_through() {
        let server = TestServer::new(make_auth_router()).unwrap();
        let auth_value =
            axum::http::HeaderValue::from_str(&format!("Bearer {TEST_TOKEN_HEX}")).unwrap();
        let resp = server
            .get("/test")
            .add_header(axum::http::header::AUTHORIZATION, auth_value)
            .await;
        assert_eq!(resp.status_code(), 200);
    }
}
