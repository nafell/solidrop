use aws_sdk_s3::Client;
use axum::{middleware::from_fn_with_state, Router};

use crate::config::AppConfig;
use crate::middleware::require_auth;

pub mod cache;
pub mod delete;
pub mod file_move;
pub mod files;
mod health;
pub mod presign;

#[derive(Clone)]
pub struct AppState {
    pub s3: Client,
    pub config: AppConfig,
}

/// Router without auth — used by integration tests that need to test auth behavior.
pub fn router() -> Router<AppState> {
    let authenticated = Router::new()
        .merge(presign::router())
        .merge(files::router())
        .merge(delete::router())
        .merge(file_move::router())
        .merge(cache::router());

    Router::new().merge(health::router()).merge(authenticated)
}

/// Build the router with auth middleware applied to all endpoints except health.
pub fn router_with_auth(state: AppState) -> Router<AppState> {
    let authenticated = Router::new()
        .merge(presign::router())
        .merge(files::router())
        .merge(delete::router())
        .merge(file_move::router())
        .merge(cache::router())
        .route_layer(from_fn_with_state(state, require_auth));

    Router::new().merge(health::router()).merge(authenticated)
}
