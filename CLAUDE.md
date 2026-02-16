# Solidrop — iPad Art Data Infrastructure

## Project Overview

Personal data infrastructure for iPad drawing workflows: encrypted backup to AWS S3, cross-device file transfer (iPad/PC), and local storage management via LRU cache eviction. Single-user system — no multi-tenancy.

See `README.md` for the full requirements and design specification (Japanese).

## Architecture

- **API Server** (`crates/api-server/`): Rust/axum HTTP server that issues S3 presigned URLs, lists files, and manages cache state. Deployed via Docker on XServer VPS.
- **Crypto Library** (`crates/crypto/`): Shared crate for AES-256-GCM encryption/decryption, Argon2id key derivation, SHA-256 hashing. Used by both server and CLI.
- **PC CLI** (`crates/cli/`): Rust CLI tool (`solidrop`) for uploading, downloading, listing, and syncing files from a PC.
- **Flutter App** (`flutter/solidrop/`): iPad/Android client (not yet created).
- **Infrastructure** (`infra/terraform/`): Terraform configs for AWS S3 bucket + IAM.

## Development Commands

```bash
# Build the entire workspace
cargo build

# Build a specific crate
cargo build -p solidrop-crypto
cargo build -p solidrop-api-server
cargo build -p solidrop-cli

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p solidrop-crypto

# Run clippy linter
cargo clippy --all-targets

# Format code
cargo fmt --all

# Check formatting without modifying
cargo fmt --all -- --check

# Run the API server locally (requires env vars)
S3_BUCKET=test API_KEY=test cargo run -p solidrop-api-server

# Run the CLI
cargo run -p solidrop-cli -- --help
```

## Key Design Decisions

1. **Client-side encryption**: All file data is AES-256-GCM encrypted before upload. The server never sees plaintext. This means no server-side thumbnails or previews.
2. **Presigned URLs**: The API server only issues S3 presigned URLs — actual file data flows directly between client and S3, never through the API server.
3. **No database (MVP)**: S3 `ListObjects` + object metadata tags serve as the source of truth. Client-side SQLite manages local cache state.
4. **LRU cache strategy**: iPad local storage treated as a cache with LRU eviction. Eviction candidates require user approval before deletion.

## Encrypted File Format

```
[Header: 45 bytes]
  Magic:    "SOLIDROP\x01" (8 bytes)
  Version:  u8 (1 byte)
  Salt:     [u8; 16] (16 bytes)
  Nonce:    [u8; 12] (12 bytes)
  OrigSize: u64 LE (8 bytes)
[Body]
  AES-256-GCM ciphertext + authentication tag
```

## Code Style

- Rust 2021 edition
- Use `thiserror` for library error types, `anyhow` for binary error handling
- Follow standard Rust naming conventions (snake_case for functions/variables, CamelCase for types)
- Keep modules focused — one responsibility per file
- Write tests in `#[cfg(test)]` modules within source files
