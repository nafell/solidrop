# artsync-cli — Specification

PC command-line tool for uploading, downloading, listing, and syncing files with the ArtSync cloud. Shares the `artsync-crypto` crate with the API server for encryption/decryption.

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
| Upload command | `src/commands/upload.rs` | **Stub** |
| Download command | `src/commands/download.rs` | **Stub** |
| List command | `src/commands/list.rs` | **Stub** |
| Sync command | `src/commands/sync.rs` | **Stub** |
| Delete command | — | **Not started** |
| Move command | — | **Not started** |

## CLI Interface

Binary name: `art-sync`

```
art-sync upload <file_path>           # Encrypt and upload a file
art-sync download <remote_path>       # Download and decrypt a file
art-sync list [--prefix <prefix>]     # List remote files
art-sync sync                         # Download new/updated files
```

Additional commands specified in README §6.2 but not yet scaffolded:
```
art-sync delete <remote_path>         # Delete a remote file
art-sync move <from> <to>             # Move file (active ↔ archived)
```

## Configuration

TOML config file loaded from the platform-specific config directory:

```toml
[server]
endpoint = "https://your-vps-domain.com/api/v1"
api_key_env = "ARTSYNC_API_KEY"       # Name of env var holding the API key

[storage]
download_dir = "~/Art/synced"         # Where downloaded files are saved
upload_dir = "~/Art/to-upload"        # Default upload source directory

[crypto]
keychain_service = "artsync"          # OS credential store service name
keychain_account = "master-key"       # OS credential store account name
```

**Config path** is resolved via the `directories` crate:
- Linux: `~/.config/art-sync/config.toml`
- macOS: `~/Library/Application Support/com.artsync.art-sync/config.toml`
- Windows: `C:\Users\<user>\AppData\Roaming\artsync\art-sync\config\config.toml`

**Decision: Config path via `directories` crate — TENTATIVE.** The org/app identifiers (`com`, `artsync`, `art-sync`) were chosen during scaffolding. The README specifies a TOML config but doesn't prescribe the path resolution mechanism.

**Decision: API key via environment variable — THOUGHT-THROUGH.** The config file stores the *name* of the env var (not the key itself), preventing accidental key exposure in config files. Defined in README §11.3.

## Planned Command Flows

These flows are defined in README §5.2 and documented as TODOs in the stub files.

### Upload (`art-sync upload <file_path>`)

1. Read the file from disk
2. Compute SHA-256 hash of the plaintext
3. Encrypt with AES-256-GCM using the master key
4. Send `POST /api/v1/presign/upload` with `{ path, content_hash, size_bytes }`
5. PUT the encrypted data to S3 via the returned presigned URL

### Download (`art-sync download <remote_path>`)

1. Send `POST /api/v1/presign/download` with `{ path }`
2. GET the encrypted data from S3 via the returned presigned URL
3. Decrypt with AES-256-GCM using the master key
4. Verify SHA-256 hash matches the stored content_hash
5. Save the plaintext file to the configured download directory

### List (`art-sync list [--prefix <prefix>]`)

1. Send `GET /api/v1/files?prefix=<prefix>` to the API server
2. Display the file list (path, size, last modified date)

### Sync (`art-sync sync`)

1. Send `GET /api/v1/files` to get the full remote file list
2. Compare with local files in the download directory
3. Download any new or updated files (by content_hash comparison)

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
| `artsync-crypto` | path | Shared encryption library |
| `anyhow` | 1 | Error handling (binary crate) |
| `clap` | 4 (derive) | CLI argument parsing |
| `reqwest` | 0.12 (rustls-tls) | HTTP client for API calls |
| `serde` / `serde_json` | 1 | JSON serialization |
| `tokio` | 1 (full) | Async runtime |
| `toml` | 0.8 | Config file parsing |
| `tracing` / `tracing-subscriber` | 0.1 / 0.3 | Structured logging |
| `thiserror` | 1 | Error type derives |
| `directories` | 5 | Platform-specific config paths |
