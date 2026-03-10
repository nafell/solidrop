import { useFiles } from '../hooks/useFiles'
import DownloadButton from './DownloadButton'
import type { FileEntry } from '../api/types'

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`
}

function formatDate(iso: string): string {
  return new Date(iso).toLocaleString('ja-JP', {
    year: 'numeric', month: '2-digit', day: '2-digit',
    hour: '2-digit', minute: '2-digit',
  })
}

function FileRow({ file }: { file: FileEntry }) {
  const name = file.key.split('/').pop() ?? file.key
  return (
    <tr>
      <td className="file-name" title={file.key}>{name}</td>
      <td className="file-size">{formatBytes(file.size)}</td>
      <td className="file-date">{formatDate(file.last_modified)}</td>
      <td><DownloadButton path={file.key} /></td>
    </tr>
  )
}

export default function FileList() {
  const { data, isLoading, isError, error, refetch } = useFiles()

  if (isLoading) {
    return <p className="status-row">読み込み中...</p>
  }

  if (isError) {
    return (
      <div className="status-row">
        <p className="error-msg">ファイル一覧の取得に失敗しました: {String(error)}</p>
        <button onClick={() => refetch()} style={{ marginTop: 12 }}>再試行</button>
      </div>
    )
  }

  const files = data?.files ?? []

  if (files.length === 0) {
    return <p className="status-row">ファイルがありません。アップロードしてください。</p>
  }

  return (
    <table className="file-table">
      <thead>
        <tr>
          <th>ファイル名</th>
          <th>サイズ</th>
          <th>更新日時</th>
          <th></th>
        </tr>
      </thead>
      <tbody>
        {files.map(file => (
          <FileRow key={file.key} file={file} />
        ))}
      </tbody>
    </table>
  )
}
