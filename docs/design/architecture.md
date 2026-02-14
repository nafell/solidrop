# ArtSync — Architecture Document

## System Purpose

ArtSync is a personal data infrastructure for iPad drawing workflows. It solves two problems:

1. **Storage offloading** — iPad has 128GB storage. Large .clip files (30MB median, 55MB max) accumulate. ArtSync treats local storage as an LRU cache backed by S3.
2. **Cross-device file transfer** — Move files between iPad and PC (via cloud) with client-side encryption.

Single-user system. No multi-tenancy, no shared access.

## High-Level Architecture

```
┌──────────────┐     ┌──────────────┐
│  iPad App    │     │   PC CLI     │
│  (Flutter)   │     │   (Rust)     │
│              │     │              │
│ encrypt/     │     │ encrypt/     │
│ decrypt      │     │ decrypt      │
│ locally      │     │ locally      │
└──────┬───────┘     └──────┬───────┘
       │                     │
       │  1. Request         │  1. Request
       │     presigned URL   │     presigned URL
       │                     │
       ▼                     ▼
┌─────────────────────────────────────┐
│       API Server (Rust/axum)        │
│       on XServer VPS (Docker)       │
│                                     │
│  - Issues S3 presigned URLs         │
│  - Lists files (S3 ListObjects)     │
│  - Computes eviction candidates     │
│  - Bearer token auth                │
│  - Never touches file data          │
└──────────────┬──────────────────────┘
               │
               │  2. Generate presigned URL
               │     (S3 API call)
               ▼
┌─────────────────────────────────────┐
│            AWS S3                   │
│                                     │
│  /active/     locally cached files  │
│  /archived/   cloud-only files      │
│  /transfer/   device transfers      │
│                                     │
│  Versioning: enabled                │
│  SSE-S3: enabled                    │
│  Lifecycle: archived/ → Glacier IR  │
└──────────────┬──────────────────────┘
               ▲
               │  3. PUT/GET encrypted
               │     data directly
               │
       ┌───────┴───────┐
       │               │
  iPad App          PC CLI
```

**Key insight:** The API server is a control plane, not a data plane. It never sees file contents. Clients encrypt locally, get a presigned URL, and transfer data directly to/from S3.

## Crate Dependency Graph

```
artsync-crypto (library)
    ▲           ▲
    │           │
    │           │
artsync-api-server    artsync-cli
(binary)              (binary)
```

`artsync-crypto` is the shared foundation. Both the server and CLI depend on it. The server and CLI have no dependency on each other.

## Component Details

### artsync-crypto

**Role:** All cryptographic operations. Pure computation, no I/O.

**Key flows:**
- Password → Argon2id → 256-bit master key
- Master key + per-file salt → HKDF-SHA256 → 256-bit file key
- Plaintext + file key → AES-256-GCM → ArtSync-format encrypted file (45-byte header + ciphertext)
- SHA-256 hashing for content deduplication

**Status:** Fully implemented with 12 passing tests. See `crates/crypto/SPEC.md`.

### artsync-api-server

**Role:** HTTP API server. Issues presigned URLs, lists files, manages cache state.

**Key design properties:**
- Stateless (no database). S3 is the source of truth.
- Thin control plane. File data never passes through.
- Single environment configuration via env vars.

**Status:** Server infrastructure complete. Route handlers are stubs. See `crates/api-server/SPEC.md`.

### artsync-cli

**Role:** PC-side file management. Upload, download, list, sync.

**Key design properties:**
- Shares crypto crate with server — identical encryption/decryption.
- Config via TOML file. Master key via OS credential store.
- Async HTTP via reqwest.

**Status:** CLI dispatch complete. Command handlers are stubs. See `crates/cli/SPEC.md`.

### Infrastructure (Terraform)

**Role:** AWS resource provisioning (S3 bucket, IAM).

**Status:** Complete and ready for `terraform apply`. See `infra/terraform/SPEC.md`.

### Flutter App (not yet created)

**Role:** iPad/Android client. File selection, encryption, upload, download, cache management, daily backup.

**Planned location:** `flutter/art_sync/`

**Open decisions (README §18.1):**
- TBD-6: BGTaskScheduler implementation
- TBD-7: S3 direct upload method (Dart HTTP vs. platform channel)
- TBD-8: Encryption via Dart reimplementation or Rust FFI to `artsync-crypto`

## Data Flow: Upload

```
1. User selects file on iPad/PC
2. Client reads file bytes
3. Client computes SHA-256 hash of plaintext     → content_hash
4. Client encrypts with AES-256-GCM              → encrypted bytes
5. Client sends POST /api/v1/presign/upload
   { path, content_hash, size_bytes }
6. Server generates S3 presigned PUT URL          → upload_url
7. Client PUTs encrypted bytes to upload_url
8. (iPad only) Client updates local SQLite cache
```

## Data Flow: Download

```
1. Client sends GET /api/v1/files                → file list
2. User selects file to download
3. Client sends POST /api/v1/presign/download
   { path }
4. Server generates S3 presigned GET URL          → download_url
5. Client GETs encrypted bytes from download_url
6. Client decrypts with AES-256-GCM              → plaintext
7. Client verifies SHA-256 hash                  → integrity check
8. Client saves plaintext to disk
```

## Data Flow: Cache Eviction (iPad)

```
1. Daily background task triggers
2. App calculates total local storage used
3. If over threshold (default 60GB):
   a. App sends POST /api/v1/cache/report
      { local_files: [{path, content_hash, last_used}], storage_limit_bytes }
   b. Server sorts by last_used ascending (LRU)
   c. Server returns evict_candidates
4. App presents candidates to user for approval
5. On approval:
   a. Verify file exists in S3 (content_hash match)
   b. Delete local copy
   c. Update SQLite: location = 'cloud_only'
```

**Decision: Approval-based eviction — THOUGHT-THROUGH.** Automatic eviction could delete a file the user planned to work on that day. Since the primary goal is drawing UX, false evictions are worse than manual confirmation. Automatic eviction is deferred to Phase 2 after usage patterns are understood.

## Security Architecture

### Encryption Layers

| Layer | Mechanism | Protects Against |
|---|---|---|
| Transport | TLS 1.3 (HTTPS) | Network eavesdropping |
| Application | Client-side AES-256-GCM | Server/cloud compromise, unauthorized S3 access |
| Storage | SSE-S3 (AWS-managed) | Physical disk access at AWS |

The application-layer encryption is the primary protection. The server and S3 never have access to plaintext file data or the encryption key.

### Key Management

```
Master password (user's memory / password manager)
  → Argon2id → Master key (256-bit)
    → Stored in: iPad Keychain / PC OS credential store
    → Never sent to server or cloud
    → Per-file keys derived via HKDF (per-file salt stored in file header)
```

**Risk: Key loss = data loss.** There is no recovery mechanism by design. This is explicitly accepted (README §9.4, RISK-1).

### API Authentication

Single Bearer token. Validated on all `/api/v1/*` endpoints.

```
Authorization: Bearer <API_KEY>
```

**Decision: Static API key — THOUGHT-THROUGH.** Single-user system. The key is generated once and stored as an environment variable on the VPS. Rotation is manual. More sophisticated auth (OAuth2, JWT) is unnecessary overhead for this use case.

### VPS Security

The VPS holds IAM access keys, making it a high-value target.

**Mitigations:**
- SSH key-only authentication (no password)
- UFW firewall: only HTTPS (443) open
- IAM permissions scoped to a single S3 bucket
- IAM key rotation (recommended 90 days, TBD-4)
- Docker isolation for the API server

## S3 Bucket Organization

```
{bucket}/
├── active/           Files that exist both locally and in cloud
│   └── {YYYY-MM}/    Month-based organization (convention, not enforced)
├── archived/         Files evicted from local storage (cloud-only)
│   └── {YYYY-MM}/    Transitions to Glacier Instant Retrieval after 90 days
└── transfer/         Short-lived device-to-device transfers
    └── {YYYY-MM-DD}/
```

File naming: `<original-name>.enc` (e.g., `illustration-01.clip.enc`).

**Decision: Directory structure as convention — THOUGHT-THROUGH.** The system does not enforce this structure. Clients can use arbitrary paths. The prefixes (`active/`, `archived/`, `transfer/`) are documented conventions, and only `archived/` has a Terraform lifecycle rule attached.

## Technology Choices

| Choice | Selected | Rationale | Decision Type |
|---|---|---|---|
| Server language | Rust | Primary learning objective | THOUGHT-THROUGH |
| Server framework | axum 0.7 | Tokio-native, well-maintained, ergonomic | THOUGHT-THROUGH |
| Mobile framework | Flutter | Cross-platform (iPad + Android) | THOUGHT-THROUGH |
| Cloud storage | AWS S3 | Learning objective; 99.999999999% durability | THOUGHT-THROUGH |
| Infrastructure as Code | Terraform | Standard for AWS resource management | THOUGHT-THROUGH |
| API server hosting | XServer VPS (Docker) | Existing contract, zero additional cost | THOUGHT-THROUGH |
| Local DB (iPad) | SQLite | Standard embedded DB for mobile cache state | THOUGHT-THROUGH |
| Encryption algorithm | AES-256-GCM | Industry standard AEAD; user requirement for self-only decryption | THOUGHT-THROUGH |
| KDF | Argon2id + HKDF | Argon2id for password→key; HKDF for per-file derivation | THOUGHT-THROUGH |
| CLI HTTP client | reqwest (rustls) | Widely used, async, avoids OpenSSL dependency | TENTATIVE |
| CLI config location | `directories` crate | Platform-standard config paths | TENTATIVE |
| Dockerfile base | rust:1.93-slim | Current stable toolchain | TENTATIVE |

## Development Phases

From README §17. The scaffold commit covers Phase 0 foundations.

| Phase | Scope | Key Deliverables |
|---|---|---|
| **0** (current) | Validation & foundations | Terraform, crypto crate, presigned URL proof-of-concept, iPad file behavior investigation |
| **1** | MVP | Full API server, CLI tool, Flutter app, cache management, daily backup |
| **2** | Enhancement | S3 version management UI, auto-eviction, Fargate migration consideration, PC GUI |
| **3** | UX | File Provider Extension (Swift), LAN P2P, Android app |

## Open Questions (TBDs)

These are explicitly deferred decisions from README §18.1 that affect the architecture:

| ID | Question | Impact |
|---|---|---|
| TBD-1 | S3 bucket name | Terraform apply blocked until decided |
| TBD-2 | VPS domain / TLS method | API server deployment blocked until decided |
| TBD-5 | Argon2id parameters | Performance on iPad; currently using defaults |
| TBD-8 | Flutter encryption: Dart or Rust FFI | Determines whether `artsync-crypto` is shared with mobile or reimplemented |

## Cross-References

- Full requirements and design rationale: `README.md`
- Per-crate specifications: `crates/*/SPEC.md`, `infra/terraform/SPEC.md`
- Implementation status and decision log: `docs/progress/00-init-stub-report.md`
