# solidrop-api-server — Specification

Rust/axum HTTP server deployed on XServer VPS via Docker. Issues S3 presigned URLs, lists files, and manages cache state. The server never handles file data — data flows directly between clients and S3.

## Responsibility

1. Authenticate requests (Bearer token)
2. Generate S3 presigned URLs for upload and download
3. List files from S3 (with metadata tag extraction)
4. Manage file operations (delete, move between active/archived)
5. Compute LRU eviction candidates from iPad cache reports

The server is a thin orchestration layer. It holds IAM credentials and translates client requests into S3 API calls or presigned URLs.

## Current Implementation Status

| Component | File | Status |
|---|---|---|
| Server bootstrap | `src/main.rs` | Complete |
| Config (env vars) | `src/config.rs` | Complete |
| Error responses | `src/error.rs` | Complete |
| S3 client init | `src/s3_client.rs` | Complete (custom endpoint + path-style support) |
| Route aggregation | `src/routes/mod.rs` | Complete |
| Health check | `src/routes/health.rs` | Complete |
| Presigned URLs | `src/routes/presign.rs` | **Stub** — types defined, returns empty URLs |
| File listing | `src/routes/files.rs` | **Stub** — types defined, returns empty list |
| Auth middleware | — | **Not started** |
| Delete endpoint | — | **Not started** |
| Move endpoint | — | **Not started** |
| Cache report | — | **Not started** |

## API Endpoints

Defined in README Section 7.2. The following table shows the full target API surface and current scaffold state.

| Method | Path | Purpose | Scaffolded |
|---|---|---|---|
| `GET` | `/health` | Health check | Yes (complete) |
| `POST` | `/api/v1/presign/upload` | Presigned upload URL | Yes (stub) |
| `POST` | `/api/v1/presign/download` | Presigned download URL | Yes (stub) |
| `GET` | `/api/v1/files` | List files from S3 | Yes (stub) |
| `DELETE` | `/api/v1/files/{encoded_path}` | Delete a file | No |
| `POST` | `/api/v1/files/move` | Move file (active ↔ archived) | No |
| `POST` | `/api/v1/cache/report` | iPad cache state report + eviction candidates | No |

### Request/Response Structures (defined in code)

**Presign Upload:**
- Request: `{ path: String, content_hash: String, size_bytes: u64 }`
- Response: `{ url: String, expires_in: u64 }`

**Presign Download:**
- Request: `{ path: String }`
- Response: `{ url: String, expires_in: u64 }`

**File Listing:**
- Response: `{ files: [{ path, size_bytes, last_modified, content_hash }], next_token: Option<String> }`

**Error Response (all endpoints):**
- `{ error: { code: String, message: String } }`
- HTTP status codes: 400, 401, 404, 409, 500

## Configuration

Loaded from environment variables in `config.rs`:

| Variable | Required | Default | Purpose |
|---|---|---|---|
| `PORT` | No | `3000` | Listen port |
| `S3_BUCKET` | Yes | — | S3 bucket name |
| `API_KEY` | Yes | — | Bearer token for authentication |
| `AWS_REGION` | No | `ap-northeast-1` | AWS region |
| `S3_ENDPOINT_URL` | No | — | Custom S3 endpoint (e.g. `http://minio:9000` for local dev) |
| `S3_FORCE_PATH_STYLE` | No | `false` | Path-style S3 addressing (required for MinIO) |
| `S3_PUBLIC_ENDPOINT_URL` | No | — | Public endpoint for presigned URL rewriting |

AWS credentials (`AWS_ACCESS_KEY_ID`, `AWS_SECRET_ACCESS_KEY`) are handled by the AWS SDK's standard credential chain, passed through in `docker-compose.yml`.

When `S3_ENDPOINT_URL` is unset, the AWS SDK uses standard AWS S3 endpoints (production behavior). When set, the S3 client connects to the specified endpoint, enabling local development with MinIO or other S3-compatible storage.

## Application State

```rust
struct AppState {
    s3: aws_sdk_s3::Client,
    config: AppConfig,
}
```

Shared across all route handlers via axum's `State` extractor.

## Design Decisions

### Presigned URL Architecture — THOUGHT-THROUGH

**Decision:** The server issues presigned URLs; file data never passes through the server.

**Rationale (README 2.2 §8):** The VPS is outside AWS. Proxying file data would mean VPS-to-S3 bandwidth costs and increased latency. With presigned URLs, the server only generates a URL (~1KB per request), and clients upload/download directly to/from S3. This makes VPS bandwidth usage negligible.

### No Database — THOUGHT-THROUGH

**Decision:** No DynamoDB, Firestore, or PostgreSQL. S3 ListObjects + object metadata tags are the source of truth.

**Rationale (README 2.2 §5):** Single user, file-path-keyed data. Adding a database would increase operational complexity without proportional benefit for this scale. The trade-off is that listing files requires S3 API calls (which have pagination limits and are eventually consistent), but this is acceptable for personal use.

### Bearer Token Auth — THOUGHT-THROUGH

**Decision:** Single static API key, validated on every `/api/v1/*` request.

**Rationale (README §7.1):** Single-user system. OAuth2/JWT adds complexity without benefit. The API key is stored as an environment variable on the VPS.

**Migration path (README §15.3):** If multi-device management becomes complex, consider JWT. Probability assessed as low.

### Error Response Format — THOUGHT-THROUGH

**Decision:** JSON error responses with machine-readable `code` and human-readable `message`.

**Rationale (README §7.3):** Standard pattern. The `code` field (e.g., `FILE_NOT_FOUND`) enables client-side error handling without parsing strings.

### Presigned URL Expiry — THOUGHT-THROUGH

**Decision:** 3600 seconds (1 hour).

**Rationale (README §10.1):** Long enough for large file uploads over slow connections. Short enough to limit exposure if a URL is leaked. For 55MB max file size, even a 1 Mbps connection would complete in ~7 minutes.

## Deployment

### Dockerfile

Multi-stage build:
1. **Build stage:** `rust:1.93-slim` — compiles `solidrop-api-server` in release mode. Includes a dummy CLI crate to satisfy workspace dependencies without building the full CLI.
2. **Runtime stage:** `debian:bookworm-slim` with `ca-certificates` (for HTTPS to AWS). Binary at `/usr/local/bin/solidrop-api-server`. Exposes port 3000.

**Decision: Rust version pinning — TENTATIVE.** The Dockerfile pins `rust:1.93-slim`. This will need updating as the toolchain evolves. No automated Rust version management is in place.

### docker-compose.yml

Three services for local development:
- `minio` — S3-compatible storage (ports 9000/9001)
- `minio-init` — auto-creates the development bucket
- `api-server` — the API server, depends on MinIO being ready

For production, the MinIO services are not used; the API server connects directly to AWS S3 when `S3_ENDPOINT_URL` is unset.

**Decision: `unless-stopped` restart policy — TENTATIVE.** Reasonable for a personal VPS. No health check or orchestration beyond Docker's restart.

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `solidrop-crypto` | path | Shared encryption library |
| `axum` | 0.7 | HTTP framework |
| `aws-sdk-s3` | 1 | S3 API client |
| `aws-config` | 1 | AWS credential/config loading |
| `serde` / `serde_json` | 1 | JSON serialization |
| `tokio` | 1 (full) | Async runtime |
| `tower-http` | 0.6 | CORS, tracing middleware |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | Structured logging |
| `thiserror` | 1 | Error type derives |

Dev-only: `axum-test` 16 (HTTP testing harness).

**Decision: tower-http 0.6 — TENTATIVE.** README specifies 0.5, but 0.6 is required for axum 0.7 compatibility. Correct pragmatic choice.
