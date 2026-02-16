# 00 — Initial Stub Report

> Phase 0 scaffold commit. Tracks what was built, what remains, and which design decisions are settled vs. tentative.

## What Was Built

### Cargo Workspace

A Rust 2021 workspace (`resolver = "2"`) with three member crates:

| Crate | Package Name | Binary | Status |
|---|---|---|---|
| `crates/crypto/` | `solidrop-crypto` | (library) | Fully implemented, 12 tests passing |
| `crates/api-server/` | `solidrop-api-server` | `solidrop-api-server` | Scaffold — routes registered, handlers return stubs |
| `crates/cli/` | `solidrop-cli` | `solidrop` | Scaffold — subcommand dispatch works, handlers print placeholders |

### Crypto Library (`solidrop-crypto`)

Production-ready. Implements the full encryption pipeline defined in README Section 9:

- **Key derivation:** Argon2id (password -> 256-bit master key), HKDF-SHA256 (master key + per-file salt -> file encryption key).
- **Encryption:** AES-256-GCM with the custom SoliDrop binary file format (45-byte header: magic + version + salt + nonce + original size, followed by ciphertext + auth tag).
- **Decryption:** Header parsing, key re-derivation, AES-256-GCM decryption, original-size verification.
- **Hashing:** SHA-256 with `sha256:<hex>` output format, verification helper.

Test coverage: encrypt/decrypt roundtrip, wrong-key rejection, truncated-data rejection, invalid-magic rejection, deterministic key derivation, salt uniqueness, file-key derivation, hash format, hash verification.

### API Server (`solidrop-api-server`)

Axum 0.7 HTTP server. The server infrastructure is complete; route handler bodies are stubs.

| Component | Status |
|---|---|
| `main.rs` — tracing init, config load, S3 client, router assembly, TCP listen | Complete |
| `config.rs` — env-var loading (PORT, S3_BUCKET, API_KEY, AWS_REGION) | Complete |
| `error.rs` — AppError enum with IntoResponse (JSON error bodies) | Complete |
| `s3_client.rs` — AWS SDK client initialization | Complete |
| `routes/health.rs` — `GET /health` | Complete |
| `routes/presign.rs` — `POST /api/v1/presign/upload`, `POST /api/v1/presign/download` | **Stub** — request/response types defined, returns empty URL |
| `routes/files.rs` — `GET /api/v1/files` | **Stub** — response type defined, returns empty list |

Missing endpoints (not yet scaffolded):
- `DELETE /api/v1/files/{encoded_path}`
- `POST /api/v1/files/move`
- `POST /api/v1/cache/report`
- Bearer token authentication middleware

### CLI Tool (`solidrop-cli`)

Clap 4 CLI with subcommand dispatch. Config loading is implemented; all command handlers are stubs.

| Component | Status |
|---|---|
| `main.rs` — argument parsing and subcommand dispatch | Complete |
| `config.rs` — TOML config loading (server, storage, crypto sections) | Complete |
| `commands/upload.rs` | **Stub** — prints placeholder |
| `commands/download.rs` | **Stub** — prints placeholder |
| `commands/list.rs` | **Stub** — prints placeholder |
| `commands/sync.rs` | **Stub** — prints placeholder |

Missing subcommands (not yet scaffolded):
- `delete`
- `move`

### Infrastructure (`infra/terraform/`)

Terraform configs for AWS resources. Ready for `terraform plan/apply` once `bucket_name` variable is provided.

| Resource | File | Status |
|---|---|---|
| AWS provider (ap-northeast-1) | `main.tf` | Complete |
| Input variables (region, project_name, bucket_name) | `variables.tf` | Complete |
| S3 bucket + versioning + SSE + public access block + lifecycle | `s3.tf` | Complete |
| IAM user + inline S3 policy (PutObject, GetObject, DeleteObject, ListBucket) | `iam.tf` | Complete |

Note: The IAM policy includes `CopyObject` in the README spec but was omitted from the Terraform — this is because S3 CopyObject uses the PutObject + GetObject permissions, so a separate action is unnecessary.

### Deployment

| File | Status |
|---|---|
| `crates/api-server/Dockerfile` — multi-stage build (rust:1.93-slim -> debian:bookworm-slim) | Complete |
| `docker-compose.yml` — single api-server service, env var passthrough | Complete |
| `.gitignore` — Rust, env files, IDE, OS artifacts | Complete |

### Claude Code Configuration

| File | Purpose |
|---|---|
| `CLAUDE.md` | Project context, dev commands, design decisions, code style |
| `.claude/settings.json` | SessionStart hook registration |
| `.claude/hooks/session-start.sh` | Installs clippy/rustfmt, builds workspace (web-only, synchronous) |

---

## Decision Log

Decisions are classified as:
- **Thought-through** — explicitly discussed in the README design doc with stated rationale.
- **Tentative** — reasonable defaults chosen during scaffolding without detailed analysis. May need revisiting.

### Thought-Through Decisions (from README)

| Decision | Rationale | README Section |
|---|---|---|
| Client-side AES-256-GCM encryption | User requirement: "only I can decrypt." Trade-off: no server-side previews. | 2.2 §7, §9 |
| Presigned URL architecture (no data through API server) | API server issues URLs only; data flows client ↔ S3 directly. Minimizes VPS bandwidth/egress. | 2.2 §8, §5.2 |
| No cloud-side database for MVP | S3 ListObjects + object metadata tags suffice for single-user. Avoids DynamoDB/Firestore complexity. | 2.2 §5 |
| LRU cache strategy with approval-based eviction | Auto-eviction risks deleting files the user planned to open. Approval-first aligns with "drawing UX comes first." | §12.4 |
| XServer VPS + AWS S3 hybrid | Existing VPS contract = zero additional cost. Presigned URLs mean minimal VPS ↔ S3 traffic. | 2.2 §8 |
| Rust for server + CLI; Flutter for mobile | Rust is the primary learning goal. Shared crypto crate across server and CLI. Flutter for cross-platform mobile. | 2.2 §9, §15 |
| File Provider Extension deferred to Phase 3 | High iOS development complexity, poor documentation, memory constraints. Risk too high for initial scope. | 2.2 §1 |
| P2P transfer deferred (YAGNI) | Cloud-via-presigned-URL is sufficient; P2P adds complexity on both client sides. | 2.2 §4 |
| S3 versioning enabled from Phase 1 | Low cost for personal use; preserves data for future version-management UI (Phase 2). | 2.2 §6 |
| Bearer token auth (single API key) | Single-user system; OAuth2/JWT unnecessary. | §7.1, §4 |
| Argon2id for password → master key | Industry standard for password hashing. Specific parameters (memory cost, iterations) are TBD. | §9.2, §18.1 TBD-5 |
| HKDF-SHA256 for per-file key derivation | Standard key derivation from master key with per-file salt, avoids nonce reuse across files. | §9.2 |
| Metadata stored in plaintext | User confirmed file names are not sensitive. Enables server-side listing without decryption. | §4 NF-3 |

### Tentative Decisions (made during scaffolding)

| Decision | Current Choice | Rationale | May Need Revisiting |
|---|---|---|---|
| Argon2id parameters | Library defaults (`Argon2::default()`) | Placeholder; README lists this as TBD-5. Should be tuned to target device (iPad) performance. | Yes — before production use |
| HKDF info string | `b"solidrop-file-encryption"` | Reasonable domain-separation string, but not specified in README. | Low priority |
| SHA-256 output format | `sha256:<hex>` prefix | Matches README examples (`sha256:abc123...`). Consistent. | No |
| tower-http version | 0.6 | README spec says 0.5; bumped for axum 0.7 compatibility. | No — correct choice |
| axum-test version | 16 | Latest compatible version for dev-dependencies. | No |
| reqwest TLS backend | `rustls-tls` (not native-tls) | Avoids OpenSSL dependency, simpler cross-compilation. Not discussed in README. | Low priority |
| CLI config path | `directories` crate (`ProjectDirs::from("dev", "nafell", "solidrop")`) | Standard platform-specific config location. Org/app names are tentative. | Low priority |
| Dockerfile base image | `rust:1.93-slim` | Matches current toolchain. Will need updating as Rust versions change. | As needed |
| docker-compose restart policy | `unless-stopped` | Reasonable default for personal VPS deployment. | No |

---

## What Remains (Phase 0 → Phase 1 gap)

### Must implement before Phase 1 MVP

1. **API server presigned URL generation** — The core value of the server. Requires `aws-sdk-s3` presigning API.
2. **API server file listing** — S3 ListObjects with metadata tag extraction.
3. **API server auth middleware** — Bearer token validation on all `/api/v1/*` routes.
4. **API server remaining endpoints** — DELETE, move, cache/report.
5. **CLI upload flow** — Read file → hash → encrypt → request presigned URL → PUT to S3.
6. **CLI download flow** — Request presigned URL → GET from S3 → decrypt → verify hash → save.
7. **CLI list/sync flows** — API calls + display/comparison logic.
8. **Terraform apply** — Provision actual S3 bucket and IAM user (requires `bucket_name` decision, TBD-1).
9. **VPS deployment** — Docker build, TLS setup (TBD-2), API key generation (TBD-3).

### Not yet started (Phase 1 scope but separate tracks)

- Flutter iPad/Android app (`flutter/solidrop/`)
- Local SQLite cache management (iPad-side)
- BGTaskScheduler integration (daily backup)

### Explicitly deferred (Phase 2+)

See README Section 18.2 (DEF-1 through DEF-9).
