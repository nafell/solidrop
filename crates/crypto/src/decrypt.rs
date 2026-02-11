use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};

use crate::key_derivation::derive_file_key;
use crate::{CryptoError, FORMAT_VERSION, HEADER_SIZE, MAGIC_BYTES};

struct ParsedHeader<'a> {
    salt: [u8; 16],
    nonce: [u8; 12],
    original_size: u64,
    ciphertext: &'a [u8],
}

fn parse_header(data: &[u8]) -> Result<ParsedHeader<'_>, CryptoError> {
    if data.len() < HEADER_SIZE {
        return Err(CryptoError::InvalidHeader("file too short".into()));
    }

    if &data[..8] != MAGIC_BYTES.as_slice() {
        return Err(CryptoError::InvalidHeader("invalid magic bytes".into()));
    }

    if data[8] != FORMAT_VERSION {
        return Err(CryptoError::InvalidHeader(format!(
            "unsupported version: {}",
            data[8]
        )));
    }

    let mut salt = [0u8; 16];
    salt.copy_from_slice(&data[9..25]);

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&data[25..37]);

    let original_size = u64::from_le_bytes(data[37..45].try_into().unwrap());

    Ok(ParsedHeader {
        salt,
        nonce,
        original_size,
        ciphertext: &data[HEADER_SIZE..],
    })
}

/// Decrypt an ArtSync encrypted file using the master key.
///
/// Returns the original plaintext data after verifying the AES-256-GCM authentication tag.
pub fn decrypt(master_key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, CryptoError> {
    let header = parse_header(encrypted_data)?;

    let file_key = derive_file_key(master_key, &header.salt)?;
    let nonce = Nonce::from_slice(&header.nonce);

    let cipher = Aes256Gcm::new_from_slice(&file_key)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    let plaintext = cipher
        .decrypt(nonce, header.ciphertext)
        .map_err(|e| CryptoError::DecryptionFailed(e.to_string()))?;

    if plaintext.len() as u64 != header.original_size {
        return Err(CryptoError::DecryptionFailed(format!(
            "size mismatch: header says {} bytes, got {}",
            header.original_size,
            plaintext.len()
        )));
    }

    Ok(plaintext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encrypt::encrypt;

    #[test]
    fn test_roundtrip() {
        let master_key = [42u8; 32];
        let plaintext = b"hello world, this is a test of ArtSync encryption";
        let encrypted = encrypt(&master_key, plaintext).unwrap();
        let decrypted = decrypt(&master_key, &encrypted).unwrap();
        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_wrong_key_fails() {
        let master_key = [42u8; 32];
        let wrong_key = [99u8; 32];
        let plaintext = b"secret data";
        let encrypted = encrypt(&master_key, plaintext).unwrap();
        assert!(decrypt(&wrong_key, &encrypted).is_err());
    }

    #[test]
    fn test_truncated_data_fails() {
        assert!(decrypt(&[0u8; 32], &[0u8; 10]).is_err());
    }

    #[test]
    fn test_invalid_magic_fails() {
        let mut data = vec![0u8; 100];
        data[..8].copy_from_slice(b"INVALID\x00");
        assert!(decrypt(&[0u8; 32], &data).is_err());
    }
}
