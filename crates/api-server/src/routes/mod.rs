use aws_sdk_s3::Client;
use axum::Router;

use crate::config::AppConfig;

mod files;
mod health;
mod presign;

#[derive(Clone)]
pub struct AppState {
    pub s3: Client,
    pub config: AppConfig,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .merge(health::router())
        .merge(presign::router())
        .merge(files::router())
}
