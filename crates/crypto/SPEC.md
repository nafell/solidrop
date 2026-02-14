# artsync-crypto — Specification

Shared encryption library for ArtSync. Used by both the API server and the PC CLI tool. Will also be used by the Flutter app if Rust FFI is chosen (TBD-8 in README).

## Responsibility

Provide all cryptographic primitives needed by ArtSync:

1. Password-based master key derivation (Argon2id)
2. Per-file encryption key derivation (HKDF-SHA256)
3. File encryption and decryption (AES-256-GCM) with the ArtSync binary format
4. Content hashing (SHA-256) for deduplication and integrity checks

This crate has **no network, filesystem, or async dependencies**. It operates on byte slices and returns byte vectors. Callers handle I/O.

## Public API

### Key Derivation (`key_derivation.rs`)

```rust
fn derive_master_key(password: &[u8], salt: &[u8; 16]) -> Result<[u8; 32], CryptoError>
fn derive_file_key(master_key: &[u8; 32], file_salt: &[u8; 16]) -> Result<[u8; 32], CryptoError>
fn generate_salt() -> [u8; 16]
```

**Key derivation chain:**

```
User password
  → Argon2id(password, salt) → 256-bit MasterKey
    → HKDF-SHA256(MasterKey, file_salt, info="artsync-file-encryption") → 256-bit FileKey
```

Each file gets a unique random salt, so each file gets a unique encryption key derived from the same master key. This prevents nonce reuse across files even if the same nonce value were generated twice (astronomically unlikely but defense-in-depth).

**Decision: Argon2id parameters — TENTATIVE.** Currently uses `Argon2::default()`. The README lists this as TBD-5: parameters should be tuned based on iPad hardware performance before production use. The defaults are safe but may be too slow or too fast depending on the device.

**Decision: HKDF info string — TENTATIVE.** The info string `b"artsync-file-encryption"` provides domain separation. This value was not specified in the README and was chosen during scaffolding. It is a reasonable choice and unlikely to need changing, but is not a "designed" decision.

### Encryption (`encrypt.rs`)

```rust
fn encrypt(master_key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError>
```

Takes a master key and plaintext bytes. Internally generates a random salt and nonce, derives a per-file key, encrypts with AES-256-GCM, and returns a complete ArtSync-format file (header + ciphertext + auth tag).

**Limitation:** Operates on the entire file in memory. For large files (55MB max .clip files), this is acceptable. Streaming encryption is a Phase 2+ consideration (README RISK-5).

### Decryption (`decrypt.rs`)

```rust
fn decrypt(master_key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError>
```

Parses the ArtSync header, validates magic bytes and version, re-derives the file key from the salt in the header, decrypts, and verifies the original size matches.

### Hashing (`hash.rs`)

```rust
fn sha256_hex(data: &[u8]) -> String       // Returns "sha256:<64 hex chars>"
fn verify_hash(data: &[u8], expected: &str) -> bool
```

The `sha256:` prefix format matches the content_hash format used in the API (README Section 7.2, 11.1). Hashes are computed on **plaintext**, not ciphertext — this is a deliberate design choice enabling server-side dedup without the server ever seeing plaintext data.

## ArtSync Encrypted File Format

**Decision: Custom binary format — THOUGHT-THROUGH.** Defined in README Section 9.3. Self-contained header means any file can be decrypted independently given the master key, with no external metadata required.

```
Offset  Size  Field
0       8     Magic: "ARTSYNC\x01"
8       1     Version: 0x01
9       16    Salt (for key derivation)
25      12    Nonce (for AES-256-GCM)
37      8     Original size (u64 little-endian)
45      ...   AES-256-GCM ciphertext + 16-byte auth tag
```

Total header: 45 bytes. The auth tag is appended to the ciphertext by the `aes-gcm` crate (not stored separately in the header).

### Version Field

Currently always `1`. The version field exists to allow format evolution without breaking old files. Decryption rejects any version other than `1`.

## Error Types (`error.rs`)

```rust
enum CryptoError {
    EncryptionFailed(String),
    DecryptionFailed(String),
    KeyDerivationFailed(String),
    InvalidHeader(String),
    HashMismatch { expected: String, actual: String },
}
```

Uses `thiserror` for Display/Error derive. This is a library crate, so errors are typed (not `anyhow`), following the project code style convention.

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `aes-gcm` | 0.10 | AES-256-GCM encryption/decryption |
| `argon2` | 0.5 | Argon2id password hashing |
| `hkdf` | 0.12 | HKDF-SHA256 key derivation |
| `sha2` | 0.10 | SHA-256 hashing |
| `rand` | 0.8 | Random salt/nonce generation |
| `thiserror` | 1 | Error type derives |

Dev-only: `assert_matches` 1 (not currently used in tests but available).

## Test Coverage

12 unit tests across 4 modules:

- `key_derivation`: deterministic derivation, salt variation, file key derivation, salt uniqueness (4 tests)
- `encrypt`: valid header structure, randomness across encryptions (2 tests)
- `decrypt`: encrypt/decrypt roundtrip, wrong-key rejection, truncated data, invalid magic bytes (4 tests)
- `hash`: format validation, hash verification (2 tests)

Run with: `cargo test -p artsync-crypto`
