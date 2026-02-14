pub mod decrypt;
pub mod encrypt;
pub mod hash;
pub mod key_derivation;

mod error;
pub use error::CryptoError;

/// SoliDrop encrypted file magic bytes and version.
pub const MAGIC_BYTES: &[u8; 9] = b"SOLIDROP\x01";
pub const FORMAT_VERSION: u8 = 1;

/// Header size: magic(8) + version(1) + salt(16) + nonce(12) + original_size(8) = 45 bytes
pub const HEADER_SIZE: usize = 8 + 1 + 16 + 12 + 8;
