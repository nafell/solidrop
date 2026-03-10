import { useDownload } from '../hooks/useDownload'

interface Props {
  path: string
}

export default function DownloadButton({ path }: Props) {
  const { download, downloading, error } = useDownload()

  return (
    <span>
      <button
        onClick={() => download(path)}
        disabled={downloading}
        style={{ fontSize: 12, padding: '4px 10px' }}
      >
        {downloading ? '復号中...' : 'DL'}
      </button>
      {error && <span className="error-msg" style={{ marginLeft: 8 }}>{error}</span>}
    </span>
  )
}
