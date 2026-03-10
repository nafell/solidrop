import ThumbnailCard from './ThumbnailCard'
import { isImageFile } from '../hooks/useThumbnail'
import type { FileEntry } from '../api/types'

interface Props {
  files: FileEntry[]
  onSelect: (index: number) => void
}

export default function ThumbnailGrid({ files, onSelect }: Props) {
  const imageFiles = files.filter(f => isImageFile(f.path))

  if (imageFiles.length === 0) {
    return <p className="status-row">画像ファイルがありません。</p>
  }

  return (
    <div className="thumbnail-grid">
      {imageFiles.map((file, i) => (
        <ThumbnailCard key={file.path} file={file} onClick={() => onSelect(i)} />
      ))}
    </div>
  )
}
