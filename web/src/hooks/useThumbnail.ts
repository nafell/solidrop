import { useEffect, useState } from 'react'
import { decryptFile } from '../crypto/aes'
import { getDownloadUrl } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'

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

export type ThumbnailStatus = 'idle' | 'loading' | 'ready' | 'error'

export interface ThumbnailState {
  status: ThumbnailStatus
  blobUrl: string | null
  error: string | null
}

export function useThumbnail(path: string, enabled: boolean): ThumbnailState {
  const { masterKey, apiToken } = useAuthContext()
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
      return
    }

    let cancelled = false

    setState({ status: 'loading', blobUrl: null, error: null })
    ;(async () => {
      try {
        const { url: downloadUrl } = await getDownloadUrl(apiToken, path)
        const res = await fetch(downloadUrl)
        if (!res.ok) throw new Error(`S3 GET failed: ${res.status}`)
        const encrypted = new Uint8Array(await res.arrayBuffer())
        const plaintext = await decryptFile(masterKey, encrypted)
        const blob = new Blob([plaintext as unknown as BlobPart], { type: mimeTypeFromPath(path) })
        const blobUrl = URL.createObjectURL(blob)
        blobUrlCache.set(path, blobUrl)
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
  }, [path, enabled, masterKey, apiToken])

  return state
}
