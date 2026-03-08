# Branch Review — Core Implementation

This review focuses on architectural/implementation contradictions, security concerns, runtime bug risks, error handling quality, and test completeness for the current branch.

> **Status:** All 5 findings resolved in commit `2a8cde4`.
> See `docs/progress/03-code-review-fixes-report.md` for implementation details.

## Findings

### 1) `content_hash` contract is inconsistent across API/CLI and currently unusable for integrity checks (High)

- `POST /api/v1/presign/upload` accepts `content_hash` but discards it (`let _ = (...)`).
- `GET /api/v1/files` returns `content_hash` from S3 `ETag` (ciphertext-oriented value), not the plaintext SHA-256 hash expected by the documented flow.
- CLI download flow does not verify any hash after decryption.

**Why this matters:**
The documented design relies on plaintext hash verification during download, but the current implementation cannot perform that verification end-to-end. This undermines integrity guarantees and creates a contradiction between spec and behavior.

**Suggested fix:**
- Persist plaintext hash at upload time (e.g., object metadata `x-amz-meta-content-hash`) by including signed metadata headers in presign flow.
- Return real plaintext hash in list/download metadata path (prefer `HeadObject` for per-file operations where integrity matters).
- Add CLI verification step post-decrypt (computed hash vs stored hash).

### 2) CLI `sync` and `list` only process first page; pagination token is ignored (Medium)

- API supports pagination token in `/api/v1/files` response.
- CLI client request currently supports only optional `prefix` and no continuation token input.
- `sync` uses one `list_files(None)` call and stops.

**Why this matters:**
On buckets with more than one page of objects, files beyond page 1 are silently omitted from list/sync operations.

**Suggested fix:**
Add continuation-token support in CLI API client and iterate until `next_token == None` in list/sync commands.

### 3) Upload path mapping can overwrite different source files with same basename (Medium)

- Upload remote key is always `<filename>.enc` derived from `file_name()` only.

**Why this matters:**
Uploading `projectA/sketch.clip` and `projectB/sketch.clip` maps both to `sketch.clip.enc`, causing collisions/overwrites.

**Suggested fix:**
Allow explicit remote paths, or derive remote key from relative path rooted at configured upload dir, not basename alone.

### 4) Endpoint contract drift risk between config docs and implementation (Low)

- CLI implementation appends `/api/v1/...` paths internally.
- Some docs describe endpoint examples including `/api/v1` in the base endpoint.

**Why this matters:**
If users copy an endpoint already ending in `/api/v1`, resulting URLs become `/api/v1/api/v1/...` and API calls fail.

**Suggested fix:**
Normalize base URL handling (`trim_end_matches('/')`) and either:
- enforce base host URL without `/api/v1`, or
- detect and avoid duplicate path prefixes.

### 5) Test coverage gaps for critical integration behavior (Medium)

- `api-server` reports 0 unit tests.
- No tests asserting auth middleware behavior, presign route validation, or list pagination behavior.
- CLI has key parsing tests, but no command-flow tests for upload/download/list/sync with mocked server responses.

**Why this matters:**
Recent feature work is mostly untested at route/command boundaries, increasing regression risk for auth, presign semantics, and synchronization correctness.

**Suggested fix:**
- Add API route tests (auth required, bad request validation, happy paths with mocked S3 client abstraction or integration harness).
- Add CLI integration tests with `wiremock`/`httpmock` for endpoint URL composition, pagination handling, and hash verification behavior.

## Positive Notes

- Two-client S3 presign architecture (internal API client + public-endpoint presign client) correctly addresses signature host binding concerns.
- Error response envelope is consistent and hides internal details for `Internal` errors.
