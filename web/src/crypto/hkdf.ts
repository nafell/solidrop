// HKDF-SHA256 key derivation using Web Crypto API (SubtleCrypto)
// Must produce output identical to the Rust hkdf 0.12 crate.

/**
 * Import raw key bytes as an HKDF key handle.
 */
async function importHkdfKey(keyBytes: Uint8Array): Promise<CryptoKey> {
  // new Uint8Array(src) creates a fresh ArrayBuffer-backed copy (TS 5.7 compatibility)
  return crypto.subtle.importKey('raw', new Uint8Array(keyBytes), 'HKDF', false, ['deriveBits'])
}

/**
 * Derive the API authentication token from the master key.
 *
 * Mirrors: Hkdf::<Sha256>::new(None, master_key) + expand("solidrop-api-auth", 32)
 * salt=None in Rust hkdf → 32 zero bytes (SHA-256 HashLen) per RFC 5869 §2.2
 */
export async function deriveApiToken(masterKey: Uint8Array): Promise<Uint8Array> {
  const hkdfKey = await importHkdfKey(masterKey)
  const bits = await crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(32) as Uint8Array<ArrayBuffer>, // RFC 5869: None → HashLen zeros
      info: new TextEncoder().encode('solidrop-api-auth'),
    },
    hkdfKey,
    256,
  )
  return new Uint8Array(bits)
}

/**
 * Derive a per-file encryption key from the master key and file salt.
 *
 * Mirrors: Hkdf::<Sha256>::new(Some(file_salt), master_key) + expand("solidrop-file-encryption", 32)
 */
export async function deriveFileKey(
  masterKey: Uint8Array,
  fileSalt: Uint8Array,
): Promise<CryptoKey> {
  const hkdfKey = await importHkdfKey(masterKey)
  const bits = await crypto.subtle.deriveBits(
    {
      name: 'HKDF',
      hash: 'SHA-256',
      salt: new Uint8Array(fileSalt),
      info: new TextEncoder().encode('solidrop-file-encryption'),
    },
    hkdfKey,
    256,
  )
  return crypto.subtle.importKey('raw', bits, { name: 'AES-GCM' }, false, ['encrypt', 'decrypt'])
}
