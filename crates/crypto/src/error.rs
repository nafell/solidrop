use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),

    #[error("decryption failed: {0}")]
    DecryptionFailed(String),

    #[error("key derivation failed: {0}")]
    KeyDerivationFailed(String),

    #[error("invalid file header: {0}")]
    InvalidHeader(String),

    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
}
