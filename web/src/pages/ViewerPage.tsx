import { useState } from 'react'
import { Link, useRouter } from '@tanstack/react-router'
import { useAuthContext } from '../context/AuthContext'
import { useFiles } from '../hooks/useFiles'
import { isImageFile } from '../hooks/useThumbnail'
import ThumbnailGrid from '../components/ThumbnailGrid'
import Lightbox from '../components/Lightbox'
import type { FileEntry } from '../api/types'

export default function ViewerPage() {
  const { logout } = useAuthContext()
  const router = useRouter()
  const { data, isLoading, isError, error } = useFiles()
  const [lightboxIndex, setLightboxIndex] = useState<number | null>(null)

  function handleLogout() {
    logout()
    router.navigate({ to: '/login' })
  }

  const allFiles = data?.files ?? []
  const imageFiles: FileEntry[] = allFiles.filter(f => isImageFile(f.path))

  return (
    <>
      <header className="app-header">
        <h1>SoliDrop</h1>
        <nav className="app-nav">
          <Link to="/files" activeProps={{ className: 'active' }}>一覧</Link>
          <Link to="/viewer" activeProps={{ className: 'active' }}>ビューア</Link>
        </nav>
        <button className="btn-ghost" onClick={handleLogout}>ログアウト</button>
      </header>
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
