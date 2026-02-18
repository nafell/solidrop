use axum::http::{HeaderName, HeaderValue};
use axum::Router;
use axum_test::TestServer;
use serde_json::json;

use solidrop_api_server::config::AppConfig;
use solidrop_api_server::routes::{router_with_auth, AppState};
use solidrop_api_server::s3_client::create_s3_client;

const TEST_API_KEY: &str = "test-secret-key";

/// Build a test-ready AppConfig pointing to MinIO.
fn test_config() -> AppConfig {
    AppConfig {
        port: 3000,
        s3_bucket: std::env::var("S3_BUCKET").unwrap_or_else(|_| "solidrop-dev".into()),
        api_key: TEST_API_KEY.into(),
        aws_region: "us-east-1".into(),
        s3_endpoint_url: Some(
            std::env::var("S3_ENDPOINT_URL").unwrap_or_else(|_| "http://localhost:9000".into()),
        ),
        s3_force_path_style: true,
        s3_public_endpoint_url: Some(
            std::env::var("S3_PUBLIC_ENDPOINT_URL")
                .unwrap_or_else(|_| "http://localhost:9000".into()),
        ),
    }
}

/// Build the full app router with auth middleware.
async fn test_app() -> Router {
    let config = test_config();
    let s3 = create_s3_client(&config).await;
    let state = AppState {
        s3,
        config: config.clone(),
    };
    Router::new()
        .merge(router_with_auth(state.clone()))
        .with_state(state)
}

fn auth_header() -> (HeaderName, HeaderValue) {
    (
        HeaderName::from_static("authorization"),
        HeaderValue::from_str(&format!("Bearer {TEST_API_KEY}")).unwrap(),
    )
}

// ─── Non-S3 Tests (always run) ─────────────────────────────

#[tokio::test]
async fn test_health_no_auth() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    resp.assert_json(&json!({"status": "ok"}));
}

#[tokio::test]
async fn test_401_without_token() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let resp = server
        .post("/api/v1/presign/upload")
        .json(&json!({"path": "test.enc", "content_hash": "abc", "size_bytes": 100}))
        .await;
    resp.assert_status_unauthorized();
}

#[tokio::test]
async fn test_401_with_wrong_token() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let resp = server
        .post("/api/v1/presign/upload")
        .add_header(
            HeaderName::from_static("authorization"),
            HeaderValue::from_static("Bearer wrong-token"),
        )
        .json(&json!({"path": "test.enc", "content_hash": "abc", "size_bytes": 100}))
        .await;
    resp.assert_status_unauthorized();
}

#[tokio::test]
async fn test_auth_with_valid_token_passes() {
    // With a valid token, we should NOT get 401.
    // We may get another error (e.g., S3 connection refused) — that's fine.
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/presign/upload")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({"path": "test.enc", "content_hash": "abc", "size_bytes": 100}))
        .await;

    // Should be anything except 401
    assert_ne!(
        resp.status_code().as_u16(),
        401,
        "Expected non-401 with valid token"
    );
}

#[tokio::test]
async fn test_cache_report_no_overage() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/cache/report")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "local_files": [
                {"path": "a.enc", "content_hash": "h1", "size_bytes": 100, "last_used": "2026-01-01T00:00:00Z"}
            ],
            "storage_limit_bytes": 500
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["evict_candidates"], json!([]));
}

#[tokio::test]
async fn test_cache_report_with_eviction() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/cache/report")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "local_files": [
                {"path": "new.enc", "content_hash": "h1", "size_bytes": 300, "last_used": "2026-02-01T00:00:00Z"},
                {"path": "old.enc", "content_hash": "h2", "size_bytes": 200, "last_used": "2026-01-01T00:00:00Z"},
                {"path": "mid.enc", "content_hash": "h3", "size_bytes": 150, "last_used": "2026-01-15T00:00:00Z"}
            ],
            "storage_limit_bytes": 400
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let candidates = body["evict_candidates"].as_array().unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0]["path"], "old.enc");
    assert_eq!(candidates[0]["reason"], "lru");
    assert_eq!(candidates[1]["path"], "mid.enc");
}

#[tokio::test]
async fn test_cache_report_empty_files() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/cache/report")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "local_files": [],
            "storage_limit_bytes": 1000
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["evict_candidates"], json!([]));
}

// ─── S3 Integration Tests (require MinIO) ──────────────────

#[tokio::test]
#[ignore]
async fn test_presign_upload_returns_url() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/presign/upload")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "path": "test/upload.enc",
            "content_hash": "abc123",
            "size_bytes": 1024
        }))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let url = body["upload_url"].as_str().unwrap();
    assert!(
        url.contains("test/upload.enc"),
        "URL should contain the key"
    );
    assert!(
        url.contains("X-Amz-"),
        "URL should have presigning parameters"
    );
}

#[tokio::test]
#[ignore]
async fn test_presign_download_returns_url() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/presign/download")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({"path": "test/download.enc"}))
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let url = body["download_url"].as_str().unwrap();
    assert!(url.contains("test/download.enc"));
    assert!(url.contains("X-Amz-"));
}

#[tokio::test]
#[ignore]
async fn test_list_files_empty() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .get("/api/v1/files")
        .add_query_param("prefix", "nonexistent-prefix/")
        .add_header(header_name.clone(), header_val.clone())
        .await;

    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["files"], json!([]));
    assert_eq!(body["next_token"], json!(null));
}

#[tokio::test]
#[ignore]
async fn test_upload_then_list() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();
    let (header_name, header_val) = auth_header();

    // Get presigned upload URL
    let resp = server
        .post("/api/v1/presign/upload")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "path": "integration-test/list-test.enc",
            "content_hash": "testhash123",
            "size_bytes": 11
        }))
        .await;
    resp.assert_status_ok();
    let upload_url = resp.json::<serde_json::Value>()["upload_url"]
        .as_str()
        .unwrap()
        .to_string();

    // Upload file data via presigned URL
    let client = reqwest::Client::new();
    let upload_resp = client
        .put(&upload_url)
        .header("x-amz-meta-content-hash", "testhash123")
        .header("x-amz-meta-original-size", "11")
        .body("hello world")
        .send()
        .await
        .unwrap();
    assert!(
        upload_resp.status().is_success(),
        "presigned PUT failed: {}",
        upload_resp.status()
    );

    // List files and find our upload
    let (header_name, header_val) = auth_header();
    let resp = server
        .get("/api/v1/files")
        .add_query_param("prefix", "integration-test/")
        .add_header(header_name.clone(), header_val.clone())
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    let files = body["files"].as_array().unwrap();
    assert!(
        files
            .iter()
            .any(|f| f["key"] == "integration-test/list-test.enc"),
        "uploaded file should appear in listing"
    );

    // Cleanup: delete the test file
    let (header_name, header_val) = auth_header();
    server
        .delete("/api/v1/files/integration-test/list-test.enc")
        .add_header(header_name.clone(), header_val.clone())
        .await;
}

#[tokio::test]
#[ignore]
async fn test_delete_nonexistent_returns_404() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();

    let (header_name, header_val) = auth_header();
    let resp = server
        .delete("/api/v1/files/nonexistent/file.enc")
        .add_header(header_name.clone(), header_val.clone())
        .await;

    resp.assert_status_not_found();
}

#[tokio::test]
#[ignore]
async fn test_move_file() {
    let app = test_app().await;
    let server = TestServer::new(app).unwrap();
    let (header_name, header_val) = auth_header();

    // Upload a file first
    let resp = server
        .post("/api/v1/presign/upload")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "path": "integration-test/move-source.enc",
            "content_hash": "movehash",
            "size_bytes": 9
        }))
        .await;
    resp.assert_status_ok();
    let upload_url = resp.json::<serde_json::Value>()["upload_url"]
        .as_str()
        .unwrap()
        .to_string();

    let client = reqwest::Client::new();
    client
        .put(&upload_url)
        .header("x-amz-meta-content-hash", "movehash")
        .header("x-amz-meta-original-size", "9")
        .body("move test")
        .send()
        .await
        .unwrap();

    // Move the file
    let (header_name, header_val) = auth_header();
    let resp = server
        .post("/api/v1/files/move")
        .add_header(header_name.clone(), header_val.clone())
        .json(&json!({
            "from": "integration-test/move-source.enc",
            "to": "integration-test/move-dest.enc"
        }))
        .await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["moved"], true);

    // Verify source is gone
    let (header_name, header_val) = auth_header();
    let resp = server
        .delete("/api/v1/files/integration-test/move-source.enc")
        .add_header(header_name.clone(), header_val.clone())
        .await;
    resp.assert_status_not_found();

    // Cleanup: delete destination
    let (header_name, header_val) = auth_header();
    server
        .delete("/api/v1/files/integration-test/move-dest.enc")
        .add_header(header_name.clone(), header_val.clone())
        .await;
}
