import { useFiles } from '../hooks/useFiles'
import DownloadButton from './DownloadButton'
import { formatBytes, formatDate } from '../utils/format'
import type { FileEntry } from '../api/types'

function FileRow({ file }: { file: FileEntry }) {
  const name = file.path.split('/').pop() ?? file.path
  return (
    <tr>
      <td className="file-name" title={file.path}>{name}</td>
      <td className="file-size">{formatBytes(file.size_bytes)}</td>
      <td className="file-date">{formatDate(file.last_modified)}</td>
      <td><DownloadButton path={file.path} /></td>
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
          <FileRow key={file.path} file={file} />
        ))}
      </tbody>
    </table>
  )
}
