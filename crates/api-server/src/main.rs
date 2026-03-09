use std::net::SocketAddr;

use axum::{middleware, Router};
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod error;
mod routes;
mod s3_client;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "solidrop_api_server=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = config::AppConfig::from_env();
    tracing::debug!(bucket = %config.s3_bucket, has_verifier = !config.api_key_verifier.is_empty(), "loaded app config");

    let s3 = s3_client::create_s3_client(&config).await;
    let s3_presign = s3_client::create_presigning_s3_client(&config).await;

    let state = routes::AppState {
        s3,
        s3_presign,
        config: config.clone(),
    };

    // API routes are protected by Bearer token auth.
    // The middleware is applied here (not in routes/mod.rs) because
    // `from_fn_with_state` requires an owned state instance.
    let api = routes::api_router().route_layer(middleware::from_fn_with_state(
        state.clone(),
        routes::auth_middleware,
    ));

    let app = Router::new()
        .merge(routes::health_router())
        .merge(api)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
