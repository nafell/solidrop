// Authenticated fetch wrapper.
// apiToken is the raw 32-byte token as a 64-char hex string.

import type { ApiError } from './types'

export class ApiRequestError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    message: string,
  ) {
    super(message)
    this.name = 'ApiRequestError'
  }
}

export async function apiFetch<T>(
  path: string,
  apiToken: string,
  init: RequestInit = {},
): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      'Authorization': `Bearer ${apiToken}`,
      ...init.headers,
    },
  })

  if (!res.ok) {
    let code = 'UNKNOWN'
    let message = res.statusText
    try {
      const body = await res.json() as ApiError
      code = body.error.code
      message = body.error.message
    } catch {
      // non-JSON error body
    }
    throw new ApiRequestError(res.status, code, message)
  }

  return res.json() as Promise<T>
}
