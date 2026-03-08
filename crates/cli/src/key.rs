use anyhow::{bail, Context, Result};

/// Load the 32-byte master key from the `SOLIDROP_MASTER_KEY` environment variable.
///
/// The variable must contain exactly 64 hexadecimal characters.
///
/// # Local development
/// Generate a key with:
///   openssl rand -hex 32
/// Then export it:
///   export SOLIDROP_MASTER_KEY=<64-hex-chars>
///
/// # Production
/// Keychain integration (macOS Keychain / libsecret) is planned but not yet
/// implemented. Until then, the env-var approach serves as a secure-enough
/// substitute when the shell session is properly locked down.
pub fn load_master_key() -> Result<[u8; 32]> {
    let hex = std::env::var("SOLIDROP_MASTER_KEY").context(
        "SOLIDROP_MASTER_KEY is not set. \
         Generate one with `openssl rand -hex 32` and export it.",
    )?;

    parse_hex_key(&hex)
}

fn parse_hex_key(hex: &str) -> Result<[u8; 32]> {
    if hex.len() != 64 {
        bail!(
            "SOLIDROP_MASTER_KEY must be exactly 64 hex characters (got {})",
            hex.len()
        );
    }

    let mut key = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let byte_str = std::str::from_utf8(chunk).context("invalid hex")?;
        key[i] = u8::from_str_radix(byte_str, 16)
            .with_context(|| format!("invalid hex byte '{byte_str}' at position {i}"))?;
    }
    Ok(key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_hex() {
        let hex = "a".repeat(64);
        let key = parse_hex_key(&hex).unwrap();
        assert_eq!(key, [0xaa; 32]);
    }

    #[test]
    fn reject_short_hex() {
        assert!(parse_hex_key("aabb").is_err());
    }

    #[test]
    fn reject_invalid_chars() {
        let hex = format!("{}{}", "z".repeat(2), "a".repeat(62));
        assert!(parse_hex_key(&hex).is_err());
    }
}
