# 03 — Code Review Fixes Report

> Resolved all 5 issues raised in `docs/reviews/02-core-implementation-branch-review.md`. Primary focus: end-to-end content hash integrity chain (High), pagination completeness (Medium), upload path collision avoidance (Medium), test coverage (Medium), base URL normalisation (Low).

---

## Source Review

**Review document:** `docs/reviews/02-core-implementation-branch-review.md`

All 5 findings addressed in commit `2a8cde4` on branch `feat/local-devenv`.

---

## What Was Fixed

### Fix 1 — Base URL Trailing Slash (Issue 4 · Low)

**File:** `crates/cli/src/api.rs`

`SolidropApi::new()` now calls `.trim_end_matches('/')` on the base URL before storing it. Previously, a trailing slash in the config (e.g. `http://localhost:3001/`) would produce double-slash URLs (`http://localhost:3001//api/v1/files`), silently failing all API calls. The normalisation is applied once at construction time so all methods benefit.

```rust
Self {
    base_url: base_url.trim_end_matches('/').to_string(),
    // ...
}
```

---

### Fix 2 — End-to-End content_hash Integrity Chain (Issue 1 · High)

This was the most architecturally significant fix. Four layers of code collaborate to make the integrity chain work:

#### How the chain works

```
[Upload]
CLI computes SHA-256(plaintext)
  → sends hash in presign_upload request
  → server embeds hash as signed S3 metadata constraint
       put_object().metadata("content-hash", hash)
  → CLI sends x-amz-meta-content-hash header on PUT
  → S3 enforces header presence (signed header constraint)
  → hash stored permanently in S3 object metadata

[Download]
server calls HeadObject → reads "content-hash" metadata key
  → returns content_hash in PresignDownloadResponse
  → CLI decrypts ciphertext
  → CLI computes SHA-256(plaintext)
  → CLI compares against returned content_hash → bail! on mismatch
```

AWS normalises all user-defined metadata keys to lowercase, so `"content-hash"` is used consistently at both write and read time.

#### Server changes (`crates/api-server/src/routes/presign.rs`)

- `presign_upload`: removed `let _ = (&body.content_hash, ...)` suppressor; added validation (`content_hash.is_empty()` → `400 Bad Request`); added `.metadata("content-hash", &body.content_hash)` on the presigned PUT builder. This embeds the hash as a signed header: S3 will reject any PUT that omits or mismatches `x-amz-meta-content-hash`.

- `presign_download`: added `state.s3.head_object()` call before generating the presigned GET URL; extracts `content-hash` from the returned metadata map; returns it in the new `PresignDownloadResponse`:

  ```rust
  #[derive(Serialize)]
  struct PresignDownloadResponse {
      url: String,
      expires_in: u64,
      content_hash: Option<String>,  // None if not yet set (legacy or missing)
  }
  ```

  HeadObject uses `state.s3` (internal endpoint), not `state.s3_presign`, because it is a data API call, not a URL generation call.

- **Response type split:** The single `PresignResponse { url, expires_in }` is now two distinct structs: `PresignUploadResponse` and `PresignDownloadResponse`. They diverge in semantics and this makes the API explicit.

#### CLI changes

- **`src/api.rs`:** Matching `PresignUploadResponse { url }` and `PresignDownloadResponse { url, content_hash }`. `presign_upload` and `presign_download` return these typed responses. `put_object` gains a `content_hash: Option<&str>` parameter and sends `x-amz-meta-content-hash` when present.

- **`src/commands/upload.rs`:** Passes `Some(&content_hash)` to `put_object`.

- **`src/commands/download.rs`:** After decrypt, calls `verify_content_hash(&plaintext, expected_hash)` when `presign.content_hash.is_some()`. Bails with a descriptive error on mismatch.

  ```rust
  pub fn verify_content_hash(plaintext: &[u8], expected: &str) -> bool {
      solidrop_crypto::hash::sha256_hex(plaintext) == expected
  }
  ```

  The function is `pub` so sync (which delegates to `download::run`) also benefits automatically.

---

### Fix 3 — Upload Remote Path Collision Avoidance (Issue 3 · Medium)

**Files:** `crates/cli/src/main.rs`, `crates/cli/src/commands/upload.rs`

Added `--remote-path <REMOTE_PATH>` optional argument to the `upload` subcommand:

```
solidrop upload ./drawings/sketch.clip
  → remote key: sketch.clip.enc  (default, may collide)

solidrop upload ./drawings/sketch.clip --remote-path drawings/sketch.clip.enc
  → remote key: drawings/sketch.clip.enc  (explicit, no collision)
```

`upload::run` signature changed to `run(file_path: &str, remote_path_override: Option<&str>)`. When `Some`, the override is used directly; when `None`, the original `<filename>.enc` default applies.

---

### Fix 4 — Pagination Completeness (Issue 2 · Medium)

**Files:** `crates/cli/src/api.rs`, `crates/cli/src/commands/list.rs`, `crates/cli/src/commands/sync.rs`

`list_files` now accepts `continuation_token: Option<&str>` and appends it as a query parameter when present. Both `list` and `sync` commands collect all pages before processing:

```rust
let mut all_files = Vec::new();
let mut next_token: Option<String> = None;
loop {
    let resp = ctx.api.list_files(prefix, next_token.as_deref()).await?;
    all_files.extend(resp.files);
    next_token = resp.next_token;
    if next_token.is_none() { break; }
}
```

Previously, `list` printed a note "(more results available; pagination not yet supported)" and `sync` silently missed all files beyond page 1.

---

### Fix 5 — Test Coverage (Issue 5 · Medium)

Added `httpmock = "0.7"` to `[dev-dependencies]` in `crates/cli/Cargo.toml`.

**Test summary: 27 tests total, all pass.**

| Crate | Tests Added | What They Verify |
|---|---|---|
| `solidrop-api-server` | 3 | Auth middleware: no header → 401, wrong token → 401, correct token → 200 |
| `solidrop-api-server` | 3 | Presign validation: empty path → 400, empty content_hash → 400, download empty path → 400 |
| `solidrop-cli` | 2 | Base URL trim: slash stripped / no-slash unchanged |
| `solidrop-cli` | 2 | Pagination: continuation_token sent as query param / first page omits it |
| `solidrop-cli` | 2 | Hash verification: correct hash passes / wrong hash fails |

**Key implementation note for API server tests:** `aws_sdk_s3::Client` requires `BehaviorVersion` to be set at construction time even for tests where no S3 call is made. Test helpers use `SdkConfig::builder().behavior_version(BehaviorVersion::latest()).build()` to produce a valid (but uncredentialed) client sufficient to populate `AppState`.

---

## Decision Log

### HeadObject per download vs. ETag in listing — THOUGHT-THROUGH

**Decision:** Integrity hash is retrieved via `HeadObject` in `presign_download`, not from `ListObjectsV2` ETag. The `list` command continues to return the ETag as `content_hash` for browsing purposes.

**Rationale:** Making listing accurate would require N `HeadObject` calls per page (one per file), which is expensive and unnecessary for the list display use case. Per-file integrity matters at download time, where one `HeadObject` is acceptable and already required to construct the metadata-aware response. This keeps the `content_hash` field on `FileEntry` as an ETag-based approximate value; if precise pre-download integrity checks are needed later, a dedicated `stat` subcommand can be added.

### Signed metadata constraint at upload — THOUGHT-THROUGH

**Decision:** Server embeds `content-hash` in the presigned PUT via `.metadata("content-hash", hash)`. This makes the hash a signed constraint: S3 rejects PUTs that omit `x-amz-meta-content-hash` or provide the wrong value.

**Rationale:** Without this, a client could skip sending the header and the hash would not be stored in S3, silently breaking the integrity chain on subsequent downloads. Making it a signed constraint turns a soft convention into a hard protocol requirement enforced by S3's signature verification.

**Trade-off:** The CLI `put_object` method *must* send the header; forgetting to pass `content_hash` to `put_object` would cause a `403 SignatureDoesNotMatch`. The current code always passes `Some(&content_hash)` from `upload::run`.

### verify_content_hash as extracted function — THOUGHT-THROUGH

**Decision:** Hash verification is extracted to `pub fn verify_content_hash(plaintext: &[u8], expected: &str) -> bool` rather than inlined in `download::run`.

**Rationale:** Makes the logic independently testable with pure inputs (no I/O, no async). Also makes it clear that `sync` inherits the verification behaviour by delegating to `download::run` — no separate wiring needed.

---

## Files Changed

| File | Change |
|---|---|
| `crates/api-server/src/routes/presign.rs` | metadata on PUT presign; HeadObject on download; two response types; validation; 3 tests |
| `crates/api-server/src/routes/mod.rs` | 3 auth middleware tests |
| `crates/cli/src/api.rs` | base_url trim; PresignUpload/DownloadResponse split; put_object content_hash; list_files continuation_token; 4 tests |
| `crates/cli/src/commands/upload.rs` | remote_path_override param; pass content_hash to put_object |
| `crates/cli/src/commands/download.rs` | verify_content_hash(); integrity check; 2 tests |
| `crates/cli/src/commands/list.rs` | pagination loop |
| `crates/cli/src/commands/sync.rs` | pagination loop |
| `crates/cli/src/main.rs` | --remote-path arg for upload subcommand |
| `crates/cli/Cargo.toml` | httpmock = "0.7" dev-dependency |

---

## Verification

```
cargo test --all    → 27 passed, 0 failed
cargo clippy --all-targets → No errors
cargo fmt --all -- --check → Clean
```

---

## What Remains

### Deferred from this review

| Item | Notes |
|---|---|
| HeadObject per listing for accurate `content_hash` in `list` output | Expensive (N calls); deferred to a future `stat`/inspect command |
| `DELETE /api/v1/files/{path}` endpoint | Phase 1 |
| `POST /api/v1/files/move` endpoint | Phase 1 |
| Keychain integration for master key | Phase 1 (before production) |
| Local SQLite state for sync (LRU cache tracking) | Phase 1 |
| CLI end-to-end integration tests (full upload/download flow with mocked S3) | Future |
