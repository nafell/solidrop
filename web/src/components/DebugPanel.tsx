import { useDebugContext, type DownloadTiming, type ThumbnailTiming, type UploadTiming } from '../context/DebugContext'

function fmtMs(ms: number): string {
  if (ms >= 1000) return `${(ms / 1000).toFixed(1)} s`
  return `${Math.round(ms)} ms`
}

function fmtBytes(bytes: number): string {
  if (bytes >= 1024 * 1024) return `${(bytes / 1024 / 1024).toFixed(1)} MB`
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(0)} KB`
  return `${bytes} B`
}

function shortName(filename: string): string {
  const name = filename.split('/').pop() ?? filename
  return name.length > 20 ? name.slice(0, 18) + '…' : name
}

function UploadTable({ rows }: { rows: UploadTiming[] }) {
  if (rows.length === 0) return <p style={{ color: 'var(--text-muted)', fontSize: 11 }}>なし</p>
  return (
    <table className="debug-table">
      <thead>
        <tr>
          <th style={{ textAlign: 'left' }}>ファイル</th>
          <th>サイズ</th>
          <th>読込</th>
          <th>暗号化</th>
          <th>Presign</th>
          <th>ネット</th>
          <th>合計</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <td title={r.filename}>{shortName(r.filename)}</td>
            <td>{fmtBytes(r.fileSizeBytes)}</td>
            <td>{fmtMs(r.readMs)}</td>
            <td>{fmtMs(r.encryptMs)}</td>
            <td>{fmtMs(r.presignMs)}</td>
            <td>{fmtMs(r.networkMs)}</td>
            <td>{fmtMs(r.totalMs)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function DownloadTable({ rows }: { rows: DownloadTiming[] }) {
  if (rows.length === 0) return <p style={{ color: 'var(--text-muted)', fontSize: 11 }}>なし</p>
  return (
    <table className="debug-table">
      <thead>
        <tr>
          <th style={{ textAlign: 'left' }}>ファイル</th>
          <th>サイズ</th>
          <th>Presign</th>
          <th>ネット</th>
          <th>復号</th>
          <th>合計</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <td title={r.filename}>{shortName(r.filename)}</td>
            <td>{fmtBytes(r.encryptedBytes)}</td>
            <td>{fmtMs(r.presignMs)}</td>
            <td>{fmtMs(r.networkMs)}</td>
            <td>{fmtMs(r.decryptMs)}</td>
            <td>{fmtMs(r.totalMs)}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

function ThumbnailTable({ rows }: { rows: ThumbnailTiming[] }) {
  if (rows.length === 0) return <p style={{ color: 'var(--text-muted)', fontSize: 11 }}>なし</p>
  return (
    <table className="debug-table">
      <thead>
        <tr>
          <th style={{ textAlign: 'left' }}>ファイル</th>
          <th>Presign</th>
          <th>ネット</th>
          <th>復号</th>
          <th>合計</th>
          <th>HIT</th>
        </tr>
      </thead>
      <tbody>
        {rows.map((r, i) => (
          <tr key={i}>
            <td title={r.filename}>{shortName(r.filename)}</td>
            <td>{r.cacheHit ? '—' : fmtMs(r.presignMs)}</td>
            <td>{r.cacheHit ? '—' : fmtMs(r.networkMs)}</td>
            <td>{r.cacheHit ? '—' : fmtMs(r.decryptMs)}</td>
            <td>{r.cacheHit ? '—' : fmtMs(r.totalMs)}</td>
            <td>{r.cacheHit ? '✓' : ''}</td>
          </tr>
        ))}
      </tbody>
    </table>
  )
}

export default function DebugPanel({ onClose }: { onClose: () => void }) {
  const { uploads, downloads, thumbnails, cacheCount, cacheTotalBytes, clear } = useDebugContext()

  return (
    <div className="debug-panel">
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 12 }}>
        <h2>Debug</h2>
        <div style={{ display: 'flex', gap: 8 }}>
          <button className="btn-ghost" style={{ fontSize: 11, padding: '2px 8px' }} onClick={clear}>クリア</button>
          <button className="btn-ghost" style={{ fontSize: 11, padding: '2px 8px' }} onClick={onClose}>×</button>
        </div>
      </div>

      <div className="debug-section">
        <h3>キャッシュ</h3>
        <p className="debug-cache-stats">
          {cacheCount} 件 / {(cacheTotalBytes / 1024 / 1024).toFixed(1)} MB
        </p>
      </div>

      <div className="debug-section">
        <h3>アップロード (最新 {uploads.length}/10)</h3>
        <UploadTable rows={uploads} />
      </div>

      <div className="debug-section">
        <h3>ダウンロード (最新 {downloads.length}/10)</h3>
        <DownloadTable rows={downloads} />
      </div>

      <div className="debug-section">
        <h3>サムネイル (最新 {thumbnails.length}/20)</h3>
        <ThumbnailTable rows={thumbnails} />
      </div>
    </div>
  )
}
