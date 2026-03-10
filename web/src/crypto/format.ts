// SoliDrop encrypted file format
// Header: magic(8) + version(1) + salt(16) + nonce(12) + origSize_u64le(8) = 45 bytes
// Must be byte-identical to crates/crypto (see crates/crypto/src/lib.rs)

export const MAGIC = new TextEncoder().encode('SOLIDROP') // 8 bytes
export const FORMAT_VERSION = 1
export const HEADER_SIZE = 45 // 8+1+16+12+8

export interface SoliDropHeader {
  salt: Uint8Array   // 16 bytes
  nonce: Uint8Array  // 12 bytes
  origSize: number   // original plaintext size in bytes
}

export function parseHeader(data: Uint8Array): SoliDropHeader {
  if (data.length < HEADER_SIZE) {
    throw new Error(`File too short: ${data.length} bytes (minimum ${HEADER_SIZE})`)
  }

  // Verify magic bytes
  for (let i = 0; i < 8; i++) {
    if (data[i] !== MAGIC[i]) {
      throw new Error('Invalid magic bytes: not a SoliDrop encrypted file')
    }
  }

  // Verify version
  if (data[8] !== FORMAT_VERSION) {
    throw new Error(`Unsupported format version: ${data[8]} (expected ${FORMAT_VERSION})`)
  }

  const salt = data.slice(9, 25)    // 16 bytes
  const nonce = data.slice(25, 37)  // 12 bytes

  // origSize: u64 little-endian at offset 37 (8 bytes)
  const origSizeBytes = data.slice(37, 45)
  const view = new DataView(origSizeBytes.buffer, origSizeBytes.byteOffset, 8)
  // Read as two 32-bit ints (low and high) to handle u64 safely in JS
  const low = view.getUint32(0, true)
  const high = view.getUint32(4, true)
  if (high !== 0) {
    throw new Error('File size exceeds 4GB — not supported by this client')
  }
  const origSize = low

  return { salt, nonce, origSize }
}

export function buildHeader(salt: Uint8Array, nonce: Uint8Array, origSize: number): Uint8Array {
  const header = new Uint8Array(HEADER_SIZE)
  header.set(MAGIC, 0)           // magic: offset 0, 8 bytes
  header[8] = FORMAT_VERSION     // version: offset 8, 1 byte
  header.set(salt, 9)            // salt: offset 9, 16 bytes
  header.set(nonce, 25)          // nonce: offset 25, 12 bytes

  // origSize: u64 LE at offset 37
  const view = new DataView(header.buffer, 37, 8)
  view.setUint32(0, origSize & 0xffffffff, true)
  view.setUint32(4, Math.floor(origSize / 0x100000000), true)

  return header
}
