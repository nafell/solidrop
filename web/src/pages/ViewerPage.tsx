import { useState } from 'react'
import { useFiles } from '../hooks/useFiles'
import { isImageFile } from '../hooks/useThumbnail'
import AppHeader from '../components/AppHeader'
import ThumbnailGrid from '../components/ThumbnailGrid'
import Lightbox from '../components/Lightbox'
import type { FileEntry } from '../api/types'

export default function ViewerPage() {
  const { data, isLoading, isError, error } = useFiles()
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null)

  const allFiles = data?.files ?? []
  const imageFiles: FileEntry[] = allFiles.filter(f => isImageFile(f.path))

  return (
    <>
      <AppHeader />
      <main className="page">
        {isLoading && <p className="status-row">読み込み中...</p>}
        {isError && <p className="status-row error-msg">エラー: {String(error)}</p>}
        {!isLoading && !isError && (
          <ThumbnailGrid
            files={allFiles}
            onSelect={i => setLightboxIndex(i)}
          />
        )}
      </main>
      {lightboxIndex !== null && (
        <Lightbox
          files={imageFiles}
          index={lightboxIndex}
          onClose={() => setLightboxIndex(null)}
          onPrev={() => setLightboxIndex(i => Math.max(0, (i ?? 0) - 1))}
          onNext={() => setLightboxIndex(i => Math.min(imageFiles.length - 1, (i ?? 0) + 1))}
        />
      )}
    </>
  )
}
