# 01 — Local Development Environment Report

> Added MinIO-based local S3 development environment. API server and S3 storage now run fully locally via `docker compose up`.

## What Was Built

### MinIO Integration in docker-compose.yml

Three-service Docker Compose setup:

| Service | Purpose | Status |
|---|---|---|
| `minio` | S3-compatible storage (ports 9000 API, 9001 Console) | Complete |
| `minio-init` | Auto-creates `solidrop-dev` bucket via `mc mb` | Complete |
| `api-server` | Existing — updated with MinIO defaults and dependency ordering | Complete |

Added `minio-data` volume for persistence and `solidrop-network` for inter-service communication.

### API Server Config Extension (`config.rs`)

Three optional fields added to `AppConfig`:

| Field | Env Var | Default | Purpose |
|---|---|---|---|
| `s3_endpoint_url` | `S3_ENDPOINT_URL` | None | Custom S3 endpoint (e.g. `http://minio:9000`) |
| `s3_force_path_style` | `S3_FORCE_PATH_STYLE` | `false` | Path-style addressing for MinIO |
| `s3_public_endpoint_url` | `S3_PUBLIC_ENDPOINT_URL` | None | Public endpoint for presigned URL rewriting |

All optional with backward-compatible defaults — no impact on production behavior when unset.

### S3 Client Custom Endpoint Support (`s3_client.rs`)

- Switched from `Client::new()` to `S3ConfigBuilder` to support `endpoint_url()` and `force_path_style()`.
- Added `rewrite_presigned_url_for_public_access()` utility function for Docker hostname translation (e.g. `http://minio:9000` → `http://localhost:9000`).
- 2 unit tests added for the rewrite function.

### Environment Template (`.env.example`)

Documented template covering all environment variables. `.gitignore` updated with `!.env.example` to track it in git.

## Decision Log

### MinIO over LocalStack — THOUGHT-THROUGH

**Decision:** Use MinIO as the local S3 emulator.

**Rationale:** This project only needs S3 — no DynamoDB, Lambda, or other AWS services. MinIO is purpose-built for S3 compatibility, starts faster, uses less memory, and has a built-in web console for inspecting bucket contents. LocalStack would be unnecessarily heavy.

### Presigned URL Host Rewriting — THOUGHT-THROUGH

**Decision:** Separate internal (`S3_ENDPOINT_URL`) and external (`S3_PUBLIC_ENDPOINT_URL`) endpoint configuration with string replacement on generated presigned URLs.

**Rationale:** Docker networking requires the API server to reach MinIO at `http://minio:9000` (internal DNS), but clients running outside Docker need `http://localhost:9000`. The simplest correct solution is to generate the presigned URL using the internal endpoint (so the AWS SDK signs correctly) then replace the host portion before returning to the client. This avoids DNS hacks, custom signing logic, or exposing MinIO on the Docker network hostname.

### Default Values in docker-compose.yml — TENTATIVE

**Decision:** docker-compose.yml includes sensible defaults (e.g. `S3_BUCKET:-solidrop-dev`, `API_KEY:-dev-api-key`) so `docker compose up` works without a `.env` file.

**Rationale:** Reduces friction for local development. The defaults are for development only and not suitable for production. This is a convenience choice — may revisit if it causes confusion about which values are production vs. development.

## Files Changed

| File | Change |
|---|---|
| `crates/api-server/src/config.rs` | Added 3 optional S3 config fields |
| `crates/api-server/src/s3_client.rs` | Custom endpoint support + URL rewrite utility |
| `docker-compose.yml` | Added MinIO service, init container, network, volume |
| `.env.example` | New — environment variable template |
| `.gitignore` | Added `!.env.example` |

## Verification

- `cargo build` — compiles successfully
- `cargo test -p solidrop-api-server` — 2/2 new tests pass
- `cargo clippy --all-targets` — clean (only expected dead-code warnings for not-yet-implemented presigned URL handlers)
- Pre-existing `solidrop-crypto` test failures (2 tests) are unrelated to this change
