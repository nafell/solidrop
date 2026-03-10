import { apiFetch } from './client'
import type {
  PresignUploadRequest,
  PresignUploadResponse,
  PresignDownloadRequest,
  PresignDownloadResponse,
} from './types'

export async function getUploadUrl(
  apiToken: string,
  req: PresignUploadRequest,
): Promise<PresignUploadResponse> {
  return apiFetch<PresignUploadResponse>('/api/v1/presign/upload', apiToken, {
    method: 'POST',
    body: JSON.stringify(req),
  })
}

export async function getDownloadUrl(
  apiToken: string,
  path: string,
): Promise<PresignDownloadResponse> {
  return apiFetch<PresignDownloadResponse>('/api/v1/presign/download', apiToken, {
    method: 'POST',
    body: JSON.stringify({ path } satisfies PresignDownloadRequest),
  })
}

/**
 * PUT encrypted bytes directly to S3 via presigned URL.
 * Uses XMLHttpRequest to support upload progress events.
 */
export function putToS3(
  url: string,
  data: Uint8Array,
  contentHash: string,
  onProgress?: (percent: number) => void,
): Promise<void> {
  return new Promise((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('PUT', url)
    // Required: presigned URL was signed with x-amz-meta-content-hash as a
    // signed header constraint; S3 rejects PUTs that omit this header.
    xhr.setRequestHeader('x-amz-meta-content-hash', contentHash)
    if (onProgress) {
      xhr.upload.addEventListener('progress', (e) => {
        if (e.lengthComputable) onProgress((e.loaded / e.total) * 100)
      })
    }
    xhr.addEventListener('load', () => {
      if (xhr.status === 200) {
        resolve()
      } else {
        reject(new Error(`S3 PUT failed: ${xhr.status} ${xhr.statusText}`))
      }
    })
    xhr.addEventListener('error', () => reject(new Error('Network error during S3 PUT')))
    xhr.send(data as unknown as XMLHttpRequestBodyInit)
  })
}
