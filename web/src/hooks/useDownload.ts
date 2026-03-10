import { useCallback, useState } from 'react'
import { decryptFile } from '../crypto/aes'
import { getDownloadUrl } from '../api/presign'
import { useAuthContext } from '../context/AuthContext'

export interface DownloadState {
  downloading: boolean
  error: string | null
}

export function useDownload() {
  const { masterKey, apiToken } = useAuthContext()
  const [state, setState] = useState<DownloadState>({ downloading: false, error: null })

  const download = useCallback(
    async (path: string) => {
      if (!masterKey || !apiToken) return

      setState({ downloading: true, error: null })
      try {
        // 1. Get presigned download URL
        const { download_url } = await getDownloadUrl(apiToken, path)

        // 2. Fetch encrypted bytes from S3
        const res = await fetch(download_url)
        if (!res.ok) throw new Error(`S3 GET failed: ${res.status}`)
        const encrypted = new Uint8Array(await res.arrayBuffer())

        // 3. Decrypt (validates header, AES-GCM tag, and origSize)
        const plaintext = await decryptFile(masterKey, encrypted)

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
    [masterKey, apiToken],
  )

  return { download, ...state }
}
