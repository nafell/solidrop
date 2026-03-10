import { apiFetch } from './client'
import type { FilesResponse } from './types'

export async function fetchFiles(
  apiToken: string,
  prefix?: string,
  limit = 100,
): Promise<FilesResponse> {
  const params = new URLSearchParams()
  if (prefix) params.set('prefix', prefix)
  params.set('limit', String(limit))
  return apiFetch<FilesResponse>(`/api/v1/files?${params}`, apiToken)
}

export async function deleteFile(apiToken: string, path: string): Promise<void> {
  const encoded = encodeURIComponent(path)
  await apiFetch(`/api/v1/files/${encoded}`, apiToken, { method: 'DELETE' })
}
