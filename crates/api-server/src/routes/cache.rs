use axum::{routing::post, Json, Router};
use serde::{Deserialize, Serialize};

use super::AppState;

#[derive(Deserialize)]
struct CacheReportRequest {
    local_files: Vec<LocalFileEntry>,
    storage_limit_bytes: u64,
}

#[derive(Deserialize)]
struct LocalFileEntry {
    path: String,
    #[allow(dead_code)]
    content_hash: String,
    size_bytes: u64,
    last_used: String,
}

#[derive(Serialize, Deserialize)]
struct CacheReportResponse {
    evict_candidates: Vec<EvictCandidate>,
}

#[derive(Serialize, Deserialize)]
struct EvictCandidate {
    path: String,
    reason: String,
    last_used: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/cache/report", post(cache_report))
}

async fn cache_report(Json(req): Json<CacheReportRequest>) -> Json<CacheReportResponse> {
    let total_bytes: u64 = req.local_files.iter().map(|f| f.size_bytes).sum();

    if total_bytes <= req.storage_limit_bytes {
        return Json(CacheReportResponse {
            evict_candidates: vec![],
        });
    }

    let need_to_free = total_bytes - req.storage_limit_bytes;

    let mut sorted = req.local_files;
    sorted.sort_by(|a, b| a.last_used.cmp(&b.last_used));

    let mut freed: u64 = 0;
    let mut evict_candidates = Vec::new();

    for entry in sorted {
        if freed >= need_to_free {
            break;
        }
        freed += entry.size_bytes;
        evict_candidates.push(EvictCandidate {
            path: entry.path,
            reason: "lru".to_string(),
            last_used: entry.last_used,
        });
    }

    Json(CacheReportResponse { evict_candidates })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use tower::ServiceExt;

    fn test_router() -> Router {
        Router::new().route("/api/v1/cache/report", post(cache_report))
    }

    #[tokio::test]
    async fn under_limit_returns_empty() {
        let body = json!({
            "local_files": [
                {
                    "path": "a.png",
                    "content_hash": "abc",
                    "size_bytes": 100,
                    "last_used": "2026-01-01T00:00:00Z"
                }
            ],
            "storage_limit_bytes": 200
        });

        let resp = test_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cache/report")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: CacheReportResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.evict_candidates.is_empty());
    }

    #[tokio::test]
    async fn over_limit_evicts_oldest_first() {
        let body = json!({
            "local_files": [
                {
                    "path": "new.png",
                    "content_hash": "h1",
                    "size_bytes": 300,
                    "last_used": "2026-02-01T00:00:00Z"
                },
                {
                    "path": "old.png",
                    "content_hash": "h2",
                    "size_bytes": 200,
                    "last_used": "2026-01-01T00:00:00Z"
                },
                {
                    "path": "mid.png",
                    "content_hash": "h3",
                    "size_bytes": 150,
                    "last_used": "2026-01-15T00:00:00Z"
                }
            ],
            "storage_limit_bytes": 400
        });

        let resp = test_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cache/report")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: CacheReportResponse = serde_json::from_slice(&body).unwrap();

        // Total = 650, limit = 400, need to free 250.
        // Sorted by last_used: old(200) -> mid(150) -> new(300)
        // old(200) < 250 -> keep going; old(200) + mid(150) = 350 >= 250 -> stop.
        assert_eq!(result.evict_candidates.len(), 2);
        assert_eq!(result.evict_candidates[0].path, "old.png");
        assert_eq!(result.evict_candidates[0].reason, "lru");
        assert_eq!(result.evict_candidates[1].path, "mid.png");
        assert_eq!(result.evict_candidates[1].reason, "lru");
    }

    #[tokio::test]
    async fn exactly_at_limit_returns_empty() {
        let body = json!({
            "local_files": [
                {
                    "path": "a.png",
                    "content_hash": "h1",
                    "size_bytes": 500,
                    "last_used": "2026-01-01T00:00:00Z"
                }
            ],
            "storage_limit_bytes": 500
        });

        let resp = test_router()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/v1/cache/report")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_vec(&body).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let result: CacheReportResponse = serde_json::from_slice(&body).unwrap();
        assert!(result.evict_candidates.is_empty());
    }
}
