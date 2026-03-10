import { useEffect, useRef, useState } from 'react'
import { useThumbnail } from '../hooks/useThumbnail'
import type { FileEntry } from '../api/types'

interface Props {
  file: FileEntry
  onClick: () => void
}

export default function ThumbnailCard({ file, onClick }: Props) {
  const [visible, setVisible] = useState(false)
  const ref = useRef<HTMLDivElement>(null)
  const { status, blobUrl, error } = useThumbnail(file.path, visible)

  useEffect(() => {
    const el = ref.current
    if (!el) return
    const observer = new IntersectionObserver(
      ([entry]) => { if (entry.isIntersecting) setVisible(true) },
      { rootMargin: '200px' },
    )
    observer.observe(el)
    return () => observer.disconnect()
  }, [])

  const name = file.path.split('/').pop() ?? file.path

  return (
    <div ref={ref} className="thumbnail-card" onClick={onClick} title={name}>
      {status === 'idle' || status === 'loading' ? (
        <span className="thumbnail-spinner" />
      ) : status === 'error' ? (
        <span className="thumbnail-error" title={error ?? undefined}>!</span>
      ) : (
        <img src={blobUrl!} alt={name} />
      )}
    </div>
  )
}
