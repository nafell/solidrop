import { useEffect } from 'react'
import { useThumbnail } from '../hooks/useThumbnail'
import { formatBytes } from '../utils/format'
import type { FileEntry } from '../api/types'

interface Props {
  files: FileEntry[]
  index: number
  onClose: () => void
  onPrev: () => void
  onNext: () => void
}

function LightboxImage({ file }: { file: FileEntry }) {
  const { status, blobUrl, error } = useThumbnail(file.path, true)
  if (status === 'idle' || status === 'loading') {
    return <span className="thumbnail-spinner lightbox-spinner" />
  }
  if (status === 'error') {
    return <p className="error-msg">{error}</p>
  }
  return <img className="lightbox-img" src={blobUrl!} alt={file.path.split('/').pop()} />
}

export default function Lightbox({ files, index, onClose, onPrev, onNext }: Props) {
  const file = files[index]
  const name = file.path.split('/').pop() ?? file.path

  useEffect(() => {
    function handleKey(e: KeyboardEvent) {
      if (e.key === 'Escape') onClose()
      else if (e.key === 'ArrowLeft') onPrev()
      else if (e.key === 'ArrowRight') onNext()
    }
    window.addEventListener('keydown', handleKey)
    return () => window.removeEventListener('keydown', handleKey)
  }, [onClose, onPrev, onNext])

  return (
    <div className="lightbox-overlay" onClick={onClose}>
      <div className="lightbox-content" onClick={e => e.stopPropagation()}>
        <button className="lightbox-close btn-ghost" onClick={onClose}>✕</button>
        <LightboxImage file={file} />
        <div className="lightbox-footer">
          <span className="lightbox-filename">{name}</span>
          <span className="lightbox-size">{formatBytes(file.size_bytes)}</span>
        </div>
        {files.length > 1 && (
          <>
            <button className="lightbox-nav lightbox-prev btn-ghost" onClick={onPrev}>‹</button>
            <button className="lightbox-nav lightbox-next btn-ghost" onClick={onNext}>›</button>
          </>
        )}
      </div>
    </div>
  )
}
