import { useRef, type ChangeEvent } from 'react'
import { useUpload } from '../hooks/useUpload'

export default function UploadButton() {
  const inputRef = useRef<HTMLInputElement>(null)
  const { upload, uploading, progress, error } = useUpload()

  async function handleChange(e: ChangeEvent<HTMLInputElement>) {
    const file = e.target.files?.[0]
    if (!file) return
    await upload(file)
    // Reset the input so the same file can be re-uploaded
    if (inputRef.current) inputRef.current.value = ''
  }

  return (
    <div>
      <input
        ref={inputRef}
        type="file"
        style={{ display: 'none' }}
        onChange={handleChange}
        disabled={uploading}
      />
      <button onClick={() => inputRef.current?.click()} disabled={uploading}>
        {uploading ? `アップロード中... ${Math.round(progress)}%` : 'アップロード'}
      </button>
      {uploading && (
        <div className="progress-bar">
          <div className="progress-bar-fill" style={{ width: `${progress}%` }} />
        </div>
      )}
      {error && <p className="error-msg">{error}</p>}
    </div>
  )
}
