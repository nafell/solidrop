// AES-256-GCM encryption / decryption using Web Crypto API.
// Output format is byte-identical to the solidrop-crypto Rust crate.

import { buildHeader, parseHeader, HEADER_SIZE } from './format'
import { deriveFileKey } from './hkdf'

/**
 * Encrypt plaintext bytes with the master key.
 * Returns the full SoliDrop-format file (header + ciphertext + 16-byte auth tag).
 */
export async function encryptFile(
  masterKey: Uint8Array,
  plaintext: Uint8Array,
): Promise<Uint8Array> {
  const salt = crypto.getRandomValues(new Uint8Array(16))
  const nonce = crypto.getRandomValues(new Uint8Array(12))
  const fileKey = await deriveFileKey(masterKey, salt)

  const ciphertext = await crypto.subtle.encrypt(
    { name: 'AES-GCM', iv: nonce },
    fileKey,
    plaintext as unknown as Uint8Array<ArrayBuffer>,
  )

  const header = buildHeader(salt, nonce, plaintext.byteLength)
  const output = new Uint8Array(HEADER_SIZE + ciphertext.byteLength)
  output.set(header, 0)
  output.set(new Uint8Array(ciphertext), HEADER_SIZE)
  return output
}

/**
 * Decrypt a SoliDrop-format file with the master key.
 * Throws if magic/version is invalid, decryption fails, or size mismatches.
 */
export async function decryptFile(
  masterKey: Uint8Array,
  encryptedData: Uint8Array,
): Promise<Uint8Array> {
  const { salt, nonce, origSize } = parseHeader(encryptedData)
  const fileKey = await deriveFileKey(masterKey, salt)
  const body = encryptedData.slice(HEADER_SIZE)

  let plaintext: ArrayBuffer
  try {
    plaintext = await crypto.subtle.decrypt(
      { name: 'AES-GCM', iv: new Uint8Array(nonce) },
      fileKey,
      body as unknown as Uint8Array<ArrayBuffer>,
    )
  } catch {
    throw new Error('Decryption failed: wrong key or corrupted data')
  }

  if (plaintext.byteLength !== origSize) {
    throw new Error(
      `Size mismatch: header says ${origSize} bytes, got ${plaintext.byteLength}`,
    )
  }

  return new Uint8Array(plaintext)
}

/**
 * Compute SHA-256 of data and return as "sha256:<hex>" string.
 * Hash is computed on PLAINTEXT (not ciphertext) — matches Rust sha256_hex().
 */
export async function sha256Hex(data: Uint8Array): Promise<string> {
  const hashBuf = await crypto.subtle.digest('SHA-256', data as unknown as Uint8Array<ArrayBuffer>)
  const hex = Array.from(new Uint8Array(hashBuf))
    .map(b => b.toString(16).padStart(2, '0'))
    .join('')
  return `sha256:${hex}`
}
