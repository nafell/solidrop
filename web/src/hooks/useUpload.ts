import { useCallback, useState } from 'react'
import { useQueryClient } from '@tanstack/react-query'
import { encryptFile, sha256Hex } from '../crypto/aes'
import { getUploadUrl, putToS3 } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'

export interface UploadState {
  uploading: boolean
  progress: number   // 0–100
  error: string | null
}

export function useUpload() {
  const { masterKey, apiToken } = useAuthContext()
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
      try {
        // 1. Read file as ArrayBuffer
        const plaintext = new Uint8Array(await file.arrayBuffer())

        // 2. Compute content hash on plaintext
        const contentHash = await sha256Hex(plaintext)

        // 3. Encrypt
        const encrypted = await encryptFile(masterKey, plaintext)

        // 4. Get presigned upload URL
        const { upload_url } = await getUploadUrl(apiToken, {
          path: file.name,
          content_hash: contentHash,
          size_bytes: encrypted.byteLength,
        })

        // 5. PUT to S3 with progress
        await putToS3(upload_url, encrypted, (pct) => {
          setState(s => ({ ...s, progress: pct }))
        })

        // 6. Invalidate file list cache
        await queryClient.invalidateQueries({ queryKey: ['files'] })

        setState({ uploading: false, progress: 100, error: null })
      } catch (err) {
        setState({ uploading: false, progress: 0, error: String(err) })
      }
    },
    [masterKey, apiToken, queryClient],
  )

  return { upload, ...state }
}
