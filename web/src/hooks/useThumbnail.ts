import { useEffect, useState } from 'react'
import { decryptFile } from '../crypto/aes'
import { getDownloadUrl } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'
import { useDebugContext } from '../context/DebugContext'

const IMAGE_EXTENSIONS = new Set(['png', 'jpg', 'jpeg', 'webp', 'gif'])

export function isImageFile(path: string): boolean {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  return IMAGE_EXTENSIONS.has(ext)
}

function mimeTypeFromPath(path: string): string {
  const ext = path.split('.').pop()?.toLowerCase() ?? ''
  switch (ext) {
    case 'png': return 'image/png'
    case 'jpg':
    case 'jpeg': return 'image/jpeg'
    case 'webp': return 'image/webp'
    case 'gif': return 'image/gif'
    default: return 'application/octet-stream'
  }
}

// Module-level cache: survives re-mounts, lives for the tab session
const blobUrlCache = new Map<string, string>()
const blobSizeCache = new Map<string, number>()

export function getBlobCacheStats(): { count: number; totalBytes: number } {
  let total = 0
  for (const size of blobSizeCache.values()) total += size
  return { count: blobUrlCache.size, totalBytes: total }
}

export type ThumbnailStatus = 'idle' | 'loading' | 'ready' | 'error'

export interface ThumbnailState {
  status: ThumbnailStatus
  blobUrl: string | null
  error: string | null
}

export function useThumbnail(path: string, enabled: boolean): ThumbnailState {
  const { masterKey, apiToken } = useAuthContext()
  const { recordThumbnail, updateCacheStats } = useDebugContext()
  const [state, setState] = useState<ThumbnailState>(() => {
    if (blobUrlCache.has(path)) {
      return { status: 'ready', blobUrl: blobUrlCache.get(path)!, error: null }
    }
    return { status: 'idle', blobUrl: null, error: null }
  })

  useEffect(() => {
    if (!enabled) return
    if (!masterKey || !apiToken) return

    // Cache hit
    if (blobUrlCache.has(path)) {
      setState({ status: 'ready', blobUrl: blobUrlCache.get(path)!, error: null })
      recordThumbnail({
        filename: path,
        encryptedBytes: 0,
        presignMs: 0,
        networkMs: 0,
        decryptMs: 0,
        totalMs: 0,
        cacheHit: true,
        timestamp: Date.now(),
      })
      return
    }

    let cancelled = false

    setState({ status: 'loading', blobUrl: null, error: null })
    ;(async () => {
      const t0 = performance.now()
      try {
        const { url: downloadUrl } = await getDownloadUrl(apiToken, path)
        const t1 = performance.now()

        const res = await fetch(downloadUrl)
        if (!res.ok) throw new Error(`S3 GET failed: ${res.status}`)
        const encrypted = new Uint8Array(await res.arrayBuffer())
        const t2 = performance.now()

        const plaintext = await decryptFile(masterKey, encrypted)
        const t3 = performance.now()

        const blob = new Blob([plaintext as unknown as BlobPart], { type: mimeTypeFromPath(path) })
        const blobUrl = URL.createObjectURL(blob)
        blobSizeCache.set(path, blob.size)
        blobUrlCache.set(path, blobUrl)

        const stats = getBlobCacheStats()
        updateCacheStats(stats.count, stats.totalBytes)

        recordThumbnail({
          filename: path,
          encryptedBytes: encrypted.byteLength,
          presignMs: t1 - t0,
          networkMs: t2 - t1,
          decryptMs: t3 - t2,
          totalMs: t3 - t0,
          cacheHit: false,
          timestamp: Date.now(),
        })

        if (!cancelled) {
          setState({ status: 'ready', blobUrl, error: null })
        }
      } catch (err) {
        if (!cancelled) {
          setState({ status: 'error', blobUrl: null, error: String(err) })
        }
      }
    })()

    return () => { cancelled = true }
  }, [path, enabled, masterKey, apiToken, recordThumbnail, updateCacheStats])

  return state
}
