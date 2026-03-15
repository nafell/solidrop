import { useCallback, useState } from 'react'
import { decryptFile } from '../crypto/aes'
import { getDownloadUrl } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'
import { useDebugContext } from '../context/DebugContext'

export interface DownloadState {
  downloading: boolean
  error: string | null
}

export function useDownload() {
  const { masterKey, apiToken } = useAuthContext()
  const { recordDownload } = useDebugContext()
  const [state, setState] = useState<DownloadState>({ downloading: false, error: null })

  const download = useCallback(
    async (path: string) => {
      if (!masterKey || !apiToken) return

      setState({ downloading: true, error: null })
      const t0 = performance.now()
      try {
        // 1. Get presigned download URL
        const { url: downloadUrl } = await getDownloadUrl(apiToken, path)
        const t1 = performance.now()

        // 2. Fetch encrypted bytes from S3
        const res = await fetch(downloadUrl)
        if (!res.ok) throw new Error(`S3 GET failed: ${res.status}`)
        const encrypted = new Uint8Array(await res.arrayBuffer())
        const t2 = performance.now()

        // 3. Decrypt (validates header, AES-GCM tag, and origSize)
        const plaintext = await decryptFile(masterKey, encrypted)
        const t3 = performance.now()

        recordDownload({
          filename: path,
          encryptedBytes: encrypted.byteLength,
          presignMs: t1 - t0,
          networkMs: t2 - t1,
          decryptMs: t3 - t2,
          totalMs: t3 - t0,
          timestamp: Date.now(),
        })

        // 4. Trigger browser download
        const blob = new Blob([plaintext as unknown as BlobPart])
        const url = URL.createObjectURL(blob)
        const filename = path.split('/').pop() ?? 'download'
        const a = document.createElement('a')
        a.href = url
        a.download = filename
        a.click()
        URL.revokeObjectURL(url)

        setState({ downloading: false, error: null })
      } catch (err) {
        setState({ downloading: false, error: String(err) })
      }
    },
    [masterKey, apiToken, recordDownload],
  )

  return { download, ...state }
}
