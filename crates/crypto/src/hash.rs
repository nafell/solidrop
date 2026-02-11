use sha2::{Digest, Sha256};

/// Compute SHA-256 hash of the given data, returned as a hex string prefixed with "sha256:".
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("sha256:{}", hex_encode(&result))
}

/// Verify that data matches the expected hash string (format: "sha256:<hex>").
pub fn verify_hash(data: &[u8], expected: &str) -> bool {
    sha256_hex(data) == expected
}

fn hex_encode(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_hex() {
        let hash = sha256_hex(b"hello");
        assert!(hash.starts_with("sha256:"));
        assert_eq!(hash.len(), 7 + 64); // "sha256:" + 64 hex chars
    }

    #[test]
    fn test_verify_hash() {
        let data = b"hello";
        let hash = sha256_hex(data);
        assert!(verify_hash(data, &hash));
        assert!(!verify_hash(b"world", &hash));
    }
}
