// API response types — must match crates/api-server/SPEC.md

export interface FileEntry {
  path: string
  size_bytes: number
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
  url: string
}

export interface PresignDownloadRequest {
  path: string
}

export interface PresignDownloadResponse {
  url: string
  content_hash: string | null
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
