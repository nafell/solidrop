use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

use crate::key_derivation::{derive_file_key, generate_salt};
use crate::{CryptoError, FORMAT_VERSION, HEADER_SIZE, MAGIC_BYTES};

/// Encrypt plaintext data using AES-256-GCM with a derived per-file key.
///
/// Returns the full encrypted file including header (magic bytes, version, salt, nonce,
/// original size) followed by the ciphertext + authentication tag.
pub fn encrypt(master_key: &[u8; 32], plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let salt = generate_salt();
    let file_key = derive_file_key(master_key, &salt)?;

    let mut nonce_bytes = [0u8; 12];
    rand::thread_rng().fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);

    let cipher = Aes256Gcm::new_from_slice(&file_key)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|e| CryptoError::EncryptionFailed(e.to_string()))?;

    let original_size = plaintext.len() as u64;
    let mut output = Vec::with_capacity(HEADER_SIZE + ciphertext.len());

    // Header
    output.extend_from_slice(MAGIC_BYTES);
    output.push(FORMAT_VERSION);
    output.extend_from_slice(&salt);
    output.extend_from_slice(&nonce_bytes);
    output.extend_from_slice(&original_size.to_le_bytes());

    // Ciphertext + auth tag
    output.extend_from_slice(&ciphertext);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HEADER_SIZE;

    #[test]
    fn test_encrypt_produces_valid_header() {
        let master_key = [42u8; 32];
        let plaintext = b"hello world";
        let encrypted = encrypt(&master_key, plaintext).unwrap();

        assert!(encrypted.len() > HEADER_SIZE);
        assert_eq!(&encrypted[..8], MAGIC_BYTES.as_slice());
        assert_eq!(encrypted[8], FORMAT_VERSION);
    }

    #[test]
    fn test_encrypt_different_each_time() {
        let master_key = [42u8; 32];
        let plaintext = b"hello world";
        let enc1 = encrypt(&master_key, plaintext).unwrap();
        let enc2 = encrypt(&master_key, plaintext).unwrap();
        // Different salt + nonce should produce different ciphertext
        assert_ne!(enc1, enc2);
    }
}
