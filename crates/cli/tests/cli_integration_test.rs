//! Integration tests for the solidrop CLI.
//!
//! These tests require docker-compose (MinIO + API server) to be running:
//!   docker compose up -d
//!   cargo test -p solidrop-cli -- --ignored
//!   docker compose down

/// Test API endpoint (API server running via docker-compose).
const API_ENDPOINT: &str = "http://localhost:3000/api/v1";
/// Test API key matching docker-compose default.
const API_KEY: &str = "dev-api-key";

/// Build a reqwest-based API client pointing at the local docker-compose stack.
fn api_client() -> reqwest::Client {
    reqwest::Client::new()
}

/// Generate a fixed 32-byte master key for tests.
fn test_master_key() -> [u8; 32] {
    [0x42u8; 32]
}

// --- Helper functions wrapping the API (mirror api_client.rs logic) ---

async fn presign_upload(
    client: &reqwest::Client,
    path: &str,
    content_hash: &str,
    size_bytes: u64,
) -> String {
    let resp = client
        .post(format!("{API_ENDPOINT}/presign/upload"))
        .bearer_auth(API_KEY)
        .json(&serde_json::json!({
            "path": path,
            "content_hash": content_hash,
            "size_bytes": size_bytes,
        }))
        .send()
        .await
        .expect("presign_upload request failed");
    assert!(
        resp.status().is_success(),
        "presign_upload failed: {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    body["upload_url"].as_str().unwrap().to_string()
}

async fn presign_download(client: &reqwest::Client, path: &str) -> String {
    let resp = client
        .post(format!("{API_ENDPOINT}/presign/download"))
        .bearer_auth(API_KEY)
        .json(&serde_json::json!({"path": path}))
        .send()
        .await
        .expect("presign_download request failed");
    assert!(
        resp.status().is_success(),
        "presign_download failed: {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    body["download_url"].as_str().unwrap().to_string()
}

async fn list_files(client: &reqwest::Client, prefix: Option<&str>) -> Vec<serde_json::Value> {
    let mut url = format!("{API_ENDPOINT}/files");
    if let Some(p) = prefix {
        url = format!("{url}?prefix={p}");
    }
    let resp = client
        .get(&url)
        .bearer_auth(API_KEY)
        .send()
        .await
        .expect("list_files request failed");
    assert!(
        resp.status().is_success(),
        "list_files failed: {}",
        resp.status()
    );
    let body: serde_json::Value = resp.json().await.unwrap();
    body["files"].as_array().unwrap().clone()
}

async fn delete_file(client: &reqwest::Client, path: &str) {
    let resp = client
        .delete(format!("{API_ENDPOINT}/files/{path}"))
        .bearer_auth(API_KEY)
        .send()
        .await
        .expect("delete_file request failed");
    assert!(
        resp.status().is_success(),
        "delete_file failed: {}",
        resp.status()
    );
}

async fn move_file(client: &reqwest::Client, from: &str, to: &str) {
    let resp = client
        .post(format!("{API_ENDPOINT}/files/move"))
        .bearer_auth(API_KEY)
        .json(&serde_json::json!({"from": from, "to": to}))
        .send()
        .await
        .expect("move_file request failed");
    assert!(
        resp.status().is_success(),
        "move_file failed: {}",
        resp.status()
    );
}

/// Upload encrypted data to a given remote path. Returns the ciphertext bytes.
async fn upload_encrypted(
    client: &reqwest::Client,
    remote_path: &str,
    plaintext: &[u8],
    master_key: &[u8; 32],
) -> Vec<u8> {
    let content_hash = solidrop_crypto::hash::sha256_hex(plaintext);
    let ciphertext =
        solidrop_crypto::encrypt::encrypt(master_key, plaintext).expect("encryption failed");

    let upload_url =
        presign_upload(client, remote_path, &content_hash, ciphertext.len() as u64).await;

    let resp = client
        .put(&upload_url)
        .header("Content-Type", "application/octet-stream")
        .body(ciphertext.clone())
        .send()
        .await
        .expect("S3 PUT failed");
    assert!(
        resp.status().is_success(),
        "S3 PUT failed: {}",
        resp.status()
    );

    ciphertext
}

// --- Integration tests (require docker-compose) ---

#[tokio::test]
#[ignore]
async fn test_upload_and_list_roundtrip() {
    let client = api_client();
    let key = test_master_key();
    let plaintext = b"integration test data for upload+list";
    let remote_path = "test/integration/upload-list-test.clip.enc";

    // Upload
    upload_encrypted(&client, remote_path, plaintext, &key).await;

    // Verify it appears in list
    let files = list_files(&client, Some("test/integration/")).await;
    let found = files.iter().any(|f| f["key"].as_str() == Some(remote_path));
    assert!(found, "uploaded file not found in list: {remote_path}");

    // Cleanup
    delete_file(&client, remote_path).await;

    // Verify it's gone
    let files_after = list_files(&client, Some("test/integration/")).await;
    let still_there = files_after
        .iter()
        .any(|f| f["key"].as_str() == Some(remote_path));
    assert!(!still_there, "file should be deleted: {remote_path}");
}

#[tokio::test]
#[ignore]
async fn test_upload_and_download_roundtrip() {
    let client = api_client();
    let key = test_master_key();
    let plaintext = b"roundtrip test: upload then download and verify plaintext match";
    let remote_path = "test/integration/roundtrip-test.clip.enc";

    // Upload
    upload_encrypted(&client, remote_path, plaintext, &key).await;

    // Download
    let download_url = presign_download(&client, remote_path).await;
    let encrypted_data = client
        .get(&download_url)
        .send()
        .await
        .expect("S3 GET failed")
        .bytes()
        .await
        .expect("failed to read body");

    // Decrypt and verify
    let decrypted =
        solidrop_crypto::decrypt::decrypt(&key, &encrypted_data).expect("decryption failed");
    assert_eq!(
        decrypted, plaintext,
        "decrypted data does not match original"
    );

    // Cleanup
    delete_file(&client, remote_path).await;
}

#[tokio::test]
#[ignore]
async fn test_delete() {
    let client = api_client();
    let key = test_master_key();
    let plaintext = b"delete test data";
    let remote_path = "test/integration/delete-test.dat.enc";

    // Upload
    upload_encrypted(&client, remote_path, plaintext, &key).await;

    // Verify exists
    let files = list_files(&client, Some("test/integration/delete-")).await;
    assert!(files.iter().any(|f| f["key"].as_str() == Some(remote_path)));

    // Delete
    delete_file(&client, remote_path).await;

    // Verify gone
    let files = list_files(&client, Some("test/integration/delete-")).await;
    assert!(!files.iter().any(|f| f["key"].as_str() == Some(remote_path)));
}

#[tokio::test]
#[ignore]
async fn test_move() {
    let client = api_client();
    let key = test_master_key();
    let plaintext = b"move test data";
    let src = "test/integration/move-src.dat.enc";
    let dst = "test/integration/move-dst.dat.enc";

    // Upload to source
    upload_encrypted(&client, src, plaintext, &key).await;

    // Move
    move_file(&client, src, dst).await;

    // Verify source is gone and destination exists
    let files = list_files(&client, Some("test/integration/move-")).await;
    let keys: Vec<&str> = files.iter().filter_map(|f| f["key"].as_str()).collect();
    assert!(!keys.contains(&src), "source should be gone after move");
    assert!(keys.contains(&dst), "destination should exist after move");

    // Verify data integrity: download from new location and decrypt
    let download_url = presign_download(&client, dst).await;
    let encrypted = client
        .get(&download_url)
        .send()
        .await
        .unwrap()
        .bytes()
        .await
        .unwrap();
    let decrypted = solidrop_crypto::decrypt::decrypt(&key, &encrypted).unwrap();
    assert_eq!(decrypted, plaintext);

    // Cleanup
    delete_file(&client, dst).await;
}
