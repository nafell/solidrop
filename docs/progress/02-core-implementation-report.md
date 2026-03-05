# 02 — Core Implementation Report

> Local development environment verified and fixed. API server presign/listing/auth endpoints implemented. CLI upload/download/list/sync commands implemented. Crypto crate bug corrected. End-to-end local workflow now functional.

---

## Investigation: Local Environment Startup

Before writing any new code, the existing local development environment (`feat/local-devenv` branch, commit `74c3d99`) was audited for missing pieces and runtime issues.

### Findings

| Area | Status |
|---|---|
| MinIO service + bucket init | Ready — starts cleanly |
| API server Docker build | Build succeeds, **runtime crash-loop** |
| API server presign endpoints | Stub — `501 Not Implemented` |
| API server file listing | Stub — `501 Not Implemented` |
| CLI commands | Stub — all print placeholder text |
| `crates/crypto` tests | **2 tests failing** (`test_roundtrip`, `test_encrypt_produces_valid_header`) |
| CLI `config.example.toml` | Missing — no sample for local development |

### Root Cause: API Server Crash-Loop

The `api-server` container started and immediately exited with:

```
solidrop-api-server: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.38' not found
solidrop-api-server: /lib/x86_64-linux-gnu/libc.so.6: version `GLIBC_2.39' not found
```

**Cause:** The Dockerfile specified `rust:1.93-slim` (untagged), which had silently moved to a Debian trixie base (glibc 2.38+). The runtime stage was `debian:bookworm-slim` (glibc 2.36). The binary compiled on trixie required glibc symbols not present on bookworm.

**Fix:** Pin both builder and runtime to Debian bookworm explicitly.

### Root Cause: Crypto Roundtrip Failure

`MAGIC_BYTES` was defined as `b"SOLIDROP\x01"` (9 bytes). The `decrypt` function compared `data[..8]` (8 bytes) against `MAGIC_BYTES.as_slice()` (9 bytes). In Rust, comparing slices of different lengths always returns `false`, so the magic check unconditionally raised `InvalidHeader("invalid magic bytes")`, making every decrypt call fail.

The design intent (per CLAUDE.md spec) is that the magic field is `"SOLIDROP"` (8 bytes) with `FORMAT_VERSION` (1 byte) as a separate field. The `\x01` was incorrectly included in `MAGIC_BYTES`.

Additionally, the encrypt function was writing `MAGIC_BYTES` (9 bytes) followed by `FORMAT_VERSION` (1 byte), resulting in the byte sequence `SOLIDROP\x01\x01…` — the version byte appearing twice.

---

## What Was Built

### 1. Dockerfile — cargo-chef + GLIBC Fix

Rewrote `crates/api-server/Dockerfile` from a 2-stage to a 3-stage cargo-chef pattern:

```
Stage 1 (planner):  cargo chef prepare  →  recipe.json
Stage 2 (builder):  cargo chef cook     →  cached deps layer
                    cargo build         →  binary
Stage 3 (runtime):  copy binary only
```

Both builder and runtime pinned to `bookworm`:
- Builder: `lukemathwalker/cargo-chef:latest-rust-1.93-slim-bookworm`
- Runtime: `debian:bookworm-slim`

Added `API_PORT` variable to `docker-compose.yml` (`${API_PORT:-3000}`) and `.env.example` so the host-side port can be changed without editing the Compose file.

### 2. Crypto Crate — MAGIC_BYTES Bug Fix

Changed `MAGIC_BYTES` from `&[u8; 9]` (`b"SOLIDROP\x01"`) to `&[u8; 8]` (`b"SOLIDROP"`). All 12 unit tests now pass. No change to `HEADER_SIZE` (still 45) or any offset in `decrypt.rs` — the rest of the layout was already correct.

### 3. API Server — Presigned URLs (`routes/presign.rs`)

Implemented `POST /api/v1/presign/upload` and `POST /api/v1/presign/download` using `aws_sdk_s3::presigning::PresigningConfig` with a 3600-second expiry.

**Critical design issue discovered:** Presigned URL signatures include the `host` header. The previous approach in report 01 (generate URL with internal endpoint `minio:9000`, then string-replace the host with `localhost:9000`) silently breaks the HMAC signature. The client PUT/GET to MinIO would always return `SignatureDoesNotMatch`.

**Fix:** Added a second S3 client (`s3_presign`) in `AppState`, configured with `S3_PUBLIC_ENDPOINT_URL` (`http://localhost:9000`). The regular client (`s3`) continues to use the internal endpoint for ListObjects/HeadObject calls. Presigned URL handlers use `s3_presign` exclusively. The `rewrite_presigned_url_for_public_access` utility is now unused and has been removed.

### 4. API Server — File Listing (`routes/files.rs`)

Implemented `GET /api/v1/files` using `list_objects_v2`. Accepts optional `prefix` and `continuation_token` query parameters. Returns:

```json
{
  "files": [
    {
      "path": "drawings/sketch.procreate.enc",
      "size_bytes": 1048576,
      "last_modified": "2026-03-05T13:16:55.766Z",
      "content_hash": "d41d8cd98f00b204e9800998ecf8427e"
    }
  ],
  "next_token": null
}
```

`content_hash` is populated from the S3 ETag (the actual SHA-256 of the plaintext content is not available from `ListObjectsV2` without fetching per-object tags via separate `HeadObject` calls — deferred).

### 5. API Server — Auth Middleware (`routes/mod.rs`, `main.rs`)

Added Bearer token middleware using `axum::middleware::from_fn_with_state`. Applied to API routes only (`/api/v1/*`); `/health` remains unauthenticated.

Axum 0.7 requires an owned state instance when calling `from_fn_with_state`, which cannot be done inside a `Router<AppState>` builder function (state is not yet available). The middleware is therefore applied in `main.rs` after state creation:

```rust
let api = routes::api_router()
    .route_layer(middleware::from_fn_with_state(state.clone(), routes::auth_middleware));
```

This required splitting `router()` into `health_router()` + `api_router()`.

### 6. CLI — API Client (`src/api.rs`)

New `SolidropApi` struct wrapping `reqwest::Client`. Methods:
- `presign_upload(path, content_hash, size_bytes)` → presigned PUT URL
- `presign_download(path)` → presigned GET URL
- `list_files(prefix)` → `FilesResponse`
- `put_object(url, data)` → upload bytes to presigned URL
- `get_object(url)` → download bytes from presigned URL

### 7. CLI — Master Key Loading (`src/key.rs`)

Added `load_master_key()` that reads the `SOLIDROP_MASTER_KEY` environment variable (64 hex chars = 32 bytes). Returns an error with instructions if unset or malformed. This is a development-phase substitute for keychain integration.

### 8. CLI — Shared Context (`src/commands/mod.rs`)

Added `CmdContext` struct (`api`, `master_key`, `config`) with a `CmdContext::load()` constructor. Centralises config loading, API key retrieval, and master key loading so each command handler calls `CmdContext::load()?` once.

### 9. CLI — Commands

| Command | What it does |
|---|---|
| `upload <file_path>` | Read → SHA-256 hash → AES-256-GCM encrypt → presign → PUT to S3 |
| `download <remote_path>` | Presign → GET from S3 → AES-256-GCM decrypt → write to `storage.download_dir` |
| `list [--prefix <p>]` | `GET /api/v1/files`, renders PATH / SIZE / LAST MODIFIED table |
| `sync` | List remote files, skip those already in `download_dir`, download the rest |

Remote path for upload is `<filename>.enc` (original filename with `.enc` appended). Download strips `.enc` when writing the local file.

### 10. CLI — `config.example.toml`

Added `crates/cli/config.example.toml` with local-dev defaults. Instructions for each platform's config path are in the file header.

---

## Decision Log

### Two S3 Clients for Presigning — THOUGHT-THROUGH

**Decision:** `AppState` holds two `aws_sdk_s3::Client` instances: `s3` (internal endpoint, for API calls) and `s3_presign` (public endpoint, for generating presigned URLs).

**Rationale:** AWS Signature Version 4 includes the `host` header in the canonical request. When the SDK generates a presigned URL using the internal endpoint (`http://minio:9000`), the signature is bound to that host. Any URL rewrite changes the host, invalidating the signature. The only correct approach is to generate the presigned URL with the endpoint that clients will actually send the request to. Two clients with separate endpoint configurations solve this cleanly without any string manipulation or custom signing logic.

**Alternative rejected:** String-replace the host after generation. Breaks HMAC signature. Was the approach in the prior `01` report and has been removed.

### MAGIC_BYTES = 8 bytes — THOUGHT-THROUGH

**Decision:** `MAGIC_BYTES = b"SOLIDROP"` (8 bytes). The version byte (`FORMAT_VERSION = 1`) is a separate field.

**Rationale:** Consistent with the CLAUDE.md file format spec, which states `Magic: "SOLIDROP\x01" (8 bytes)`. The parenthetical `(8 bytes)` indicates the magic is just `"SOLIDROP"`; the `\x01` was a misread of the spec. This makes `HEADER_SIZE = 8+1+16+12+8 = 45` correct without any offset adjustment.

**Impact:** All files encrypted with the old (buggy) code used the layout `[SOLIDROP\x01][0x01][salt…]` (10 bytes for magic+version). Files encrypted with the fixed code use `[SOLIDROP][0x01][salt…]` (9 bytes). These formats are incompatible — files encrypted before the fix cannot be decrypted by the fixed code and vice versa. Since this is a pre-release development branch with no persistent user data, no migration is required.

### API_PORT Variable — TENTATIVE

**Decision:** `docker-compose.yml` uses `${API_PORT:-3000}` for the host-side port binding. Default remains 3000.

**Rationale:** Port 3000 can conflict with other development tools (open-webui, React dev servers, etc.). Making it configurable via `.env` without modifying `docker-compose.yml` is a low-friction solution. The container always listens on port 3000 internally; only the host binding changes.

### Master Key via Environment Variable — TENTATIVE

**Decision:** `SOLIDROP_MASTER_KEY` env var (64 hex chars) provides the 32-byte master key for local development. No keychain integration.

**Rationale:** The `CryptoConfig` section (`keychain_service`, `keychain_account`) anticipates OS keychain integration (macOS Keychain / libsecret on Linux). Implementing cross-platform keychain access is non-trivial and out of scope for the local development phase. The env-var approach is a safe placeholder: the variable lives in the shell session and is not written to disk unless the user explicitly exports it to `.env`.

**Limitation:** `SOLIDROP_MASTER_KEY` must be set in every new shell session. A documented `~/.bashrc`/`~/.zshrc` export is the expected workflow until keychain support is added.

### content_hash from ETag in List Response — TENTATIVE

**Decision:** `GET /api/v1/files` returns the S3 ETag as `content_hash`.

**Rationale:** `ListObjectsV2` returns ETags without additional API calls. The actual SHA-256 hash of the plaintext (computed by the CLI at upload time) would require a `HeadObject` call per file to read the custom metadata. At N files, that is N+1 API calls vs. 1. For the MVP list use-case (browsing), ETag (which is the MD5 of the ciphertext) is sufficient to detect changes. The real SHA-256 hash will be needed for integrity verification at download time — at that point a single `HeadObject` is acceptable.

### sync Without SQLite State — TENTATIVE

**Decision:** `sync` determines "already local" by checking file existence in `download_dir`. No SQLite database.

**Rationale:** Full LRU cache management requires tracking access times, eviction candidates, and a persistent local state. That is a future milestone. File-existence checking is correct for the current use case: initial population of the local cache.

---

## Files Changed

### New Files

| File | Purpose |
|---|---|
| `crates/cli/src/api.rs` | reqwest-based API client |
| `crates/cli/src/key.rs` | Master key loading from env var |
| `crates/cli/config.example.toml` | Local-dev CLI config template |

### Modified Files

| File | Change |
|---|---|
| `crates/crypto/src/lib.rs` | `MAGIC_BYTES` corrected to 8 bytes |
| `crates/api-server/Dockerfile` | cargo-chef 3-stage, pinned to bookworm |
| `crates/api-server/src/s3_client.rs` | Added `create_presigning_s3_client()`; removed `rewrite_presigned_url_for_public_access()` |
| `crates/api-server/src/routes/presign.rs` | Implemented upload + download presigning via `s3_presign` |
| `crates/api-server/src/routes/files.rs` | Implemented `ListObjectsV2`-based file listing |
| `crates/api-server/src/routes/mod.rs` | Split `router()` into `health_router()` + `api_router()`; added `auth_middleware` |
| `crates/api-server/src/main.rs` | Added `s3_presign` client; applied auth middleware |
| `crates/cli/src/commands/mod.rs` | Added `CmdContext` |
| `crates/cli/src/commands/upload.rs` | Implemented upload flow |
| `crates/cli/src/commands/download.rs` | Implemented download flow |
| `crates/cli/src/commands/list.rs` | Implemented list with table output |
| `crates/cli/src/commands/sync.rs` | Implemented sync (existence-based) |
| `crates/cli/src/main.rs` | Added `mod api`, `mod key` |
| `docker-compose.yml` | `API_PORT` variable for host port |
| `.env.example` | Added `API_PORT` |

---

## Verification

```
cargo test --workspace          → 12 passed, 0 failed (solidrop-crypto)
cargo build --workspace         → Clean (warnings only: unused struct fields, unused imports)
cargo clippy --all-targets      → No errors

docker compose up (API_PORT=3001)  → All 3 services healthy

E2E (manual, API):
  GET  /health                              → 200 {"status":"ok"}
  GET  /api/v1/files  (no auth)             → 401
  GET  /api/v1/files  (wrong key)           → 401
  GET  /api/v1/files  (correct key)         → 200 {"files":[],...}
  POST /api/v1/presign/upload               → 200 + URL pointing to localhost:9000
  PUT  <presigned URL>                      → 200 (MinIO accepts)
  GET  /api/v1/files  (after upload)        → 200, file appears in list
  POST /api/v1/presign/download             → 200 + URL
  GET  <presigned URL>                      → 200, returns file bytes

E2E (CLI):
  solidrop upload test_drawing.procreate    → "✓ Uploaded"
  solidrop list                             → shows file in table
  solidrop download test_drawing.procreate.enc → "✓ Downloaded"
  diff original vs downloaded               → identical
  solidrop sync (file already local)        → "0 downloaded, 1 already present"
```

---

## What Remains

### API Server (not yet implemented)

| Endpoint | Priority |
|---|---|
| `DELETE /api/v1/files/{encoded_path}` | Phase 1 |
| `POST /api/v1/files/move` | Phase 1 |
| `POST /api/v1/cache/report` | Phase 1 (LRU eviction) |

### CLI (not yet implemented)

| Feature | Priority |
|---|---|
| `delete` subcommand | Phase 1 |
| `move` subcommand | Phase 1 |
| Keychain integration for master key | Phase 1 (before production) |
| Local SQLite state for sync (LRU cache tracking) | Phase 1 |
| `HeadObject`-based real content_hash in list | Phase 1 (integrity verification) |

### Infrastructure

| Item | Status |
|---|---|
| AWS S3 bucket + IAM provisioning via Terraform | Pending (requires USER action) |
| VPS deployment, TLS, CI/CD | Pending (requires USER action) |
