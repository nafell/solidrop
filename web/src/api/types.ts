// API response types — must match crates/api-server/SPEC.md

export interface FileEntry {
  key: string
  size: number
  last_modified: string
  content_hash: string
}

export interface FilesResponse {
  files: FileEntry[]
  next_token: string | null
}

export interface PresignUploadRequest {
  path: string
  content_hash: string
  size_bytes: number
}

export interface PresignUploadResponse {
  upload_url: string
}

export interface PresignDownloadRequest {
  path: string
}

export interface PresignDownloadResponse {
  download_url: string
}

export interface DeleteResponse {
  deleted: true
}

export interface ApiError {
  error: {
    code: string
    message: string
  }
}
