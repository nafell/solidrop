import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { encryptFile, sha256Hex } from '../crypto/aes'
import { getUploadUrl, putToS3 } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'
import { useDebugContext } from '../context/DebugContext'

export interface UploadState {
  uploading: boolean
  progress: number   // 0–100
  error: string | null
}

export function useUpload() {
  const { masterKey, apiToken } = useAuthContext()
  const { recordUpload } = useDebugContext()
  const queryClient = useQueryClient()
  const [state, setState] = useState<UploadState>({
    uploading: false,
    progress: 0,
    error: null,
  })

  const upload = useCallback(
    async (file: File) => {
      if (!masterKey || !apiToken) return

      setState({ uploading: true, progress: 0, error: null })
      const t0 = performance.now()
      try {
        // 1. Read file as ArrayBuffer + compute content hash
        const plaintext = new Uint8Array(await file.arrayBuffer())
        const contentHash = await sha256Hex(plaintext)
        const t1 = performance.now()

        // 2. Encrypt
        const encrypted = await encryptFile(masterKey, plaintext)
        const t2 = performance.now()

        // 3. Get presigned upload URL
        const { url } = await getUploadUrl(apiToken, {
          path: file.name,
          content_hash: contentHash,
          size_bytes: encrypted.byteLength,
        })
        const t3 = performance.now()

        // 4. PUT to S3 with progress
        await putToS3(url, encrypted, contentHash, (pct) => {
          setState(s => ({ ...s, progress: pct }))
        })
        const t4 = performance.now()

        recordUpload({
          filename: file.name,
          fileSizeBytes: plaintext.byteLength,
          readMs: t1 - t0,
          encryptMs: t2 - t1,
          presignMs: t3 - t2,
          networkMs: t4 - t3,
          totalMs: t4 - t0,
          timestamp: Date.now(),
        })

        // 5. Invalidate file list cache
        await queryClient.invalidateQueries({ queryKey: ['files'] })

        setState({ uploading: false, progress: 100, error: null })
      } catch (err) {
        setState({ uploading: false, progress: 0, error: String(err) })
      }
    },
    [masterKey, apiToken, queryClient, recordUpload],
  )

  return { upload, ...state }
}
