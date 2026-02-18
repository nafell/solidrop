use axum::{routing::post, Json, Router};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::AppState;
use crate::error::AppError;

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

/// Internal representation with parsed timestamp for correct chronological sorting.
struct ParsedEntry {
    path: String,
    size_bytes: u64,
    last_used_raw: String,
    last_used: DateTime<Utc>,
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

async fn cache_report(
    Json(req): Json<CacheReportRequest>,
) -> Result<Json<CacheReportResponse>, AppError> {
    // Parse all timestamps upfront — reject the entire request on any invalid entry
    let mut parsed: Vec<ParsedEntry> = req
        .local_files
        .into_iter()
        .map(|entry| {
            let ts: DateTime<Utc> = entry
                .last_used
                .parse::<DateTime<chrono::FixedOffset>>()
                .map(|dt| dt.with_timezone(&Utc))
                .or_else(|_| entry.last_used.parse::<DateTime<Utc>>())
                .map_err(|_| {
                    AppError::BadRequest(format!(
                        "invalid last_used timestamp: {}",
                        entry.last_used
                    ))
                })?;
            Ok(ParsedEntry {
                path: entry.path,
                size_bytes: entry.size_bytes,
                last_used_raw: entry.last_used,
                last_used: ts,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;

    let total_bytes: u64 = parsed.iter().map(|f| f.size_bytes).sum();

    if total_bytes <= req.storage_limit_bytes {
        return Ok(Json(CacheReportResponse {
            evict_candidates: vec![],
        }));
    }

    let need_to_free = total_bytes - req.storage_limit_bytes;

    parsed.sort_by_key(|e| e.last_used);

    let mut freed: u64 = 0;
    let mut evict_candidates = Vec::new();

    for entry in parsed {
        if freed >= need_to_free {
            break;
        }
        freed += entry.size_bytes;
        evict_candidates.push(EvictCandidate {
            path: entry.path,
            reason: "lru".to_string(),
            last_used: entry.last_used_raw,
        });
    }

    Ok(Json(CacheReportResponse { evict_candidates }))
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
