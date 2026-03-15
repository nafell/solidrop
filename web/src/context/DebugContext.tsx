import {
  createContext,
  useCallback,
  useContext,
  useMemo,
  useState,
  type ReactNode,
} from 'react'

export interface UploadTiming {
  filename: string
  fileSizeBytes: number
  readMs: number
  encryptMs: number
  presignMs: number
  networkMs: number
  totalMs: number
  timestamp: number
}

export interface DownloadTiming {
  filename: string
  encryptedBytes: number
  presignMs: number
  networkMs: number
  decryptMs: number
  totalMs: number
  timestamp: number
}

export interface ThumbnailTiming {
  filename: string
  encryptedBytes: number
  presignMs: number
  networkMs: number
  decryptMs: number
  totalMs: number
  cacheHit: boolean
  timestamp: number
}

interface DebugState {
  uploads: UploadTiming[]
  downloads: DownloadTiming[]
  thumbnails: ThumbnailTiming[]
  cacheCount: number
  cacheTotalBytes: number
  recordUpload: (t: UploadTiming) => void
  recordDownload: (t: DownloadTiming) => void
  recordThumbnail: (t: ThumbnailTiming) => void
  updateCacheStats: (count: number, totalBytes: number) => void
  clear: () => void
}

const DebugContext = createContext<DebugState>({
  uploads: [],
  downloads: [],
  thumbnails: [],
  cacheCount: 0,
  cacheTotalBytes: 0,
  recordUpload: () => {},
  recordDownload: () => {},
  recordThumbnail: () => {},
  updateCacheStats: () => {},
  clear: () => {},
})

export function DebugProvider({ children }: { children: ReactNode }) {
  const [uploads, setUploads] = useState<UploadTiming[]>([])
  const [downloads, setDownloads] = useState<DownloadTiming[]>([])
  const [thumbnails, setThumbnails] = useState<ThumbnailTiming[]>([])
  const [cacheCount, setCacheCount] = useState(0)
  const [cacheTotalBytes, setCacheTotalBytes] = useState(0)

  const recordUpload = useCallback((t: UploadTiming) => {
    setUploads(prev => [t, ...prev].slice(0, 10))
  }, [])

  const recordDownload = useCallback((t: DownloadTiming) => {
    setDownloads(prev => [t, ...prev].slice(0, 10))
  }, [])

  const recordThumbnail = useCallback((t: ThumbnailTiming) => {
    setThumbnails(prev => [t, ...prev].slice(0, 20))
  }, [])

  const updateCacheStats = useCallback((count: number, totalBytes: number) => {
    setCacheCount(count)
    setCacheTotalBytes(totalBytes)
  }, [])

  const clear = useCallback(() => {
    setUploads([])
    setDownloads([])
    setThumbnails([])
  }, [])

  const value = useMemo<DebugState>(
    () => ({
      uploads,
      downloads,
      thumbnails,
      cacheCount,
      cacheTotalBytes,
      recordUpload,
      recordDownload,
      recordThumbnail,
      updateCacheStats,
      clear,
    }),
    [uploads, downloads, thumbnails, cacheCount, cacheTotalBytes, recordUpload, recordDownload, recordThumbnail, updateCacheStats, clear],
  )

  return <DebugContext.Provider value={value}>{children}</DebugContext.Provider>
}

export function useDebugContext(): DebugState {
  return useContext(DebugContext)
}
