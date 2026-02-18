# solidrop-cli — Specification

PC command-line tool for uploading, downloading, listing, and syncing files with the SoliDrop cloud. Shares the `solidrop-crypto` crate with the API server for encryption/decryption.

## Responsibility

Provide a PC-side interface for:

1. Encrypting and uploading files to S3 (via API server presigned URLs)
2. Downloading and decrypting files from S3
3. Listing remote files
4. Syncing new/updated files from the cloud

This is the primary PC ↔ cloud interface for Phase 1. A GUI tool is planned for Phase 2+ (README §17, Phase 2).

## Current Implementation Status

| Component | File | Status |
|---|---|---|
| CLI argument parsing | `src/main.rs` | Complete |
| Config file loading | `src/config.rs` | Complete |
| Command dispatch | `src/commands/mod.rs` | Complete |
| API client | `src/api_client.rs` | Complete |
| Master key acquisition | `src/master_key.rs` | Complete (env var; keychain planned) |
| Upload command | `src/commands/upload.rs` | Complete |
| Download command | `src/commands/download.rs` | Complete |
| List command | `src/commands/list.rs` | Complete |
| Sync command | `src/commands/sync.rs` | Complete |
| Delete command | `src/commands/delete.rs` | Complete |
| Move command | `src/commands/move_cmd.rs` | Complete |
| API contract tests | `tests/api_contract_test.rs` | Complete (requires docker-compose) |
| CLI E2E tests | — | **Not started** (TODO: `assert_cmd`) |

## CLI Interface

Binary name: `solidrop`

```
solidrop upload <file_path>           # Encrypt and upload a file
solidrop download <remote_path>       # Download and decrypt a file
solidrop list [--prefix <prefix>]     # List remote files
solidrop sync                         # Download new/updated files
solidrop delete <remote_path>         # Delete a remote file
solidrop move <from> <to>             # Move file (active ↔ archived)
```

## Configuration

TOML config file loaded from the platform-specific config directory:

```toml
[server]
endpoint = "https://your-vps-domain.com/api/v1"
api_key_env = "SOLIDROP_API_KEY"       # Name of env var holding the API key

[storage]
download_dir = "~/Art/synced"         # Where downloaded/synced files are saved

[crypto]
keychain_service = "solidrop"          # OS credential store service name
keychain_account = "master-key"       # OS credential store account name
```

**Config path** is resolved via the `directories` crate:
- Linux: `~/.config/solidrop/config.toml`
- macOS: `~/Library/Application Support/dev.nafell.solidrop/config.toml`
- Windows: `C:\Users\<user>\AppData\Roaming\nafell\solidrop\config\config.toml`

**Decision: Config path via `directories` crate — TENTATIVE.** The org/app identifiers (`dev`, `nafell`, `solidrop`) were chosen during scaffolding. The README specifies a TOML config but doesn't prescribe the path resolution mechanism.

**Decision: API key via environment variable — THOUGHT-THROUGH.** The config file stores the *name* of the env var (not the key itself), preventing accidental key exposure in config files. Defined in README §11.3.

## Command Flows

Based on README §5.2.

### Upload (`solidrop upload <file_path>`)

1. Read the file from disk
2. Compute SHA-256 hash of the plaintext
3. Encrypt with AES-256-GCM using the master key
4. Send `POST /api/v1/presign/upload` with `{ path, content_hash, size_bytes }`
5. PUT the encrypted data to S3 via the returned presigned URL

Remote path: `active/{YYYY-MM}/{filename}.enc`

### Download (`solidrop download <remote_path>`)

1. Send `POST /api/v1/presign/download` with `{ path }`
2. GET the encrypted data from S3 via the returned presigned URL
3. Decrypt with AES-256-GCM using the master key
4. Save the plaintext file (basename only) to `download_dir`

Note: content_hash verification on download is not yet implemented.

### List (`solidrop list [--prefix <prefix>]`)

1. Send `GET /api/v1/files?prefix=<prefix>` to the API server
2. Display the file list (path, size, last modified date)
3. Supports pagination via `next_token`

### Sync (`solidrop sync`)

1. Send `GET /api/v1/files?prefix=transfer/` with pagination
2. For each remote file, compute the local path by stripping the `transfer/` prefix and `.enc` suffix, preserving the directory structure under `download_dir`
3. Skip files that already exist locally
4. Download, decrypt, and save new files (creating subdirectories as needed)

Example: `transfer/2026-02-11/reference.png.enc` → `download_dir/2026-02-11/reference.png`

### Delete (`solidrop delete <remote_path>`)

1. Send `DELETE /api/v1/files/{path}` (path segments are percent-encoded)

### Move (`solidrop move <from> <to>`)

1. Send `POST /api/v1/files/move` with `{ from, to }`

## Design Decisions

### Master Key Storage — THOUGHT-THROUGH

**Decision:** Store the master key in the OS credential store (macOS Keychain, Windows Credential Manager, Linux Secret Service).

**Rationale (README §6.2, §11.3):** Avoids storing the key in plaintext config files. The key is derived from the master password via Argon2id and cached in the OS credential store after initial entry.

**Note:** The `keychain_service` / `keychain_account` fields are defined in the config, but the actual OS credential store integration is not yet implemented. This will likely require a crate like `keyring`.

### reqwest with rustls — TENTATIVE

**Decision:** Use `reqwest` with `rustls-tls` feature (not native-tls/OpenSSL).

**Rationale:** Simpler cross-compilation, no system OpenSSL dependency. Not discussed in README. This is a pragmatic choice that avoids build complexity.

### anyhow for error handling — THOUGHT-THROUGH

**Decision:** Use `anyhow` for the binary crate, `thiserror` for the library crate.

**Rationale (CLAUDE.md code style):** Standard Rust convention. Binary crates benefit from `anyhow`'s ergonomic error handling; library crates need typed errors for downstream consumers.

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `solidrop-crypto` | path | Shared encryption library |
| `anyhow` | 1 | Error handling (binary crate) |
| `chrono` | 0.4 | Timestamp formatting for upload paths |
| `clap` | 4 (derive) | CLI argument parsing |
| `directories` | 5 | Platform-specific config paths |
| `hex` | 0.4 | Master key hex decoding |
| `percent-encoding` | 2 | URL path segment encoding |
| `reqwest` | 0.12 (rustls-tls) | HTTP client for API calls |
| `serde` / `serde_json` | 1 | JSON serialization |
| `thiserror` | 1 | Error type derives |
| `tokio` | 1 (full) | Async runtime |
| `toml` | 0.8 | Config file parsing |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | Structured logging |
