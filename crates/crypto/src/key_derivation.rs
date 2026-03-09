use argon2::Argon2;
use hkdf::Hkdf;
use rand::RngCore;
use sha2::Sha256;

use crate::CryptoError;

/// Derive a 256-bit master key from a password using Argon2id.
pub fn derive_master_key(password: &[u8], salt: &[u8; 16]) -> Result<[u8; 32], CryptoError> {
    let mut master_key = [0u8; 32];
    Argon2::default()
        .hash_password_into(password, salt, &mut master_key)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(master_key)
}

/// Derive a per-file encryption key from the master key using HKDF-SHA256.
pub fn derive_file_key(
    master_key: &[u8; 32],
    file_salt: &[u8; 16],
) -> Result<[u8; 32], CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(Some(file_salt), master_key);
    let mut file_key = [0u8; 32];
    hkdf.expand(b"solidrop-file-encryption", &mut file_key)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(file_key)
}

/// Derive the API authentication token from the master key using HKDF-SHA256.
///
/// Uses info string "solidrop-api-auth" for domain separation from file keys.
/// The resulting token is sent as `Authorization: Bearer <hex>` by the client.
/// The server stores only `SHA-256(token)` — see ADR-003.
pub fn derive_api_token(master_key: &[u8; 32]) -> Result<[u8; 32], CryptoError> {
    let hkdf = Hkdf::<Sha256>::new(None, master_key);
    let mut token = [0u8; 32];
    hkdf.expand(b"solidrop-api-auth", &mut token)
        .map_err(|e| CryptoError::KeyDerivationFailed(e.to_string()))?;
    Ok(token)
}

/// Generate a random 16-byte salt.
pub fn generate_salt() -> [u8; 16] {
    let mut salt = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut salt);
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_derive_master_key_deterministic() {
        let password = b"test-password";
        let salt = [1u8; 16];
        let key1 = derive_master_key(password, &salt).unwrap();
        let key2 = derive_master_key(password, &salt).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn test_different_salts_produce_different_keys() {
        let password = b"test-password";
        let salt1 = [1u8; 16];
        let salt2 = [2u8; 16];
        let key1 = derive_master_key(password, &salt1).unwrap();
        let key2 = derive_master_key(password, &salt2).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn test_derive_file_key() {
        let master_key = [42u8; 32];
        let salt = [1u8; 16];
        let file_key = derive_file_key(&master_key, &salt).unwrap();
        assert_eq!(file_key.len(), 32);
        assert_ne!(file_key, master_key);
    }

    #[test]
    fn test_generate_salt_unique() {
        let salt1 = generate_salt();
        let salt2 = generate_salt();
        assert_ne!(salt1, salt2);
    }
}
