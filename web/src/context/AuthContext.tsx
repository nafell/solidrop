import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from 'react'
import { deriveMasterKey } from '../crypto/argon2'
import { deriveApiToken } from '../crypto/hkdf'

// sessionStorage key for persisting masterKey across page reloads (within tab)
const SESSION_KEY = 'solidrop_mk'

function toBase64(bytes: Uint8Array): string {
  return btoa(String.fromCharCode(...bytes))
}

function fromBase64(str: string): Uint8Array {
  return new Uint8Array(atob(str).split('').map(c => c.charCodeAt(0)))
}

function toHex(bytes: Uint8Array): string {
  return Array.from(bytes).map(b => b.toString(16).padStart(2, '0')).join('')
}

export interface AuthState {
  isAuthenticated: boolean
  masterKey: Uint8Array | null
  apiToken: string | null  // 64-char hex
  login: (password: string) => Promise<void>
  logout: () => void
}

const AuthContext = createContext<AuthState>({
  isAuthenticated: false,
  masterKey: null,
  apiToken: null,
  login: async () => {},
  logout: () => {},
})

export function AuthProvider({ children }: { children: ReactNode }) {
  const [masterKey, setMasterKey] = useState<Uint8Array | null>(() => {
    // Restore from sessionStorage if present (page reload within same tab)
    const stored = sessionStorage.getItem(SESSION_KEY)
    return stored ? fromBase64(stored) : null
  })
  const [apiToken, setApiToken] = useState<string | null>(null)

  // Re-derive apiToken from masterKey whenever masterKey changes
  useEffect(() => {
    if (!masterKey) {
      setApiToken(null)
      return
    }
    deriveApiToken(masterKey).then(token => setApiToken(toHex(token)))
  }, [masterKey])

  const login = useCallback(async (password: string) => {
    // Argon2id is CPU-intensive; runs on main thread (acceptable for personal tool)
    const mk = deriveMasterKey(password)
    const token = await deriveApiToken(mk)
    const tokenHex = toHex(token)

    // Verify the derived token against the server.
    // /health requires no auth, so use an authenticated endpoint instead.
    const res = await fetch('/api/v1/files?limit=1', {
      headers: { Authorization: `Bearer ${tokenHex}` },
    })
    if (res.status === 401) {
      throw new Error('パスワードが違います')
    }
    if (!res.ok) {
      throw new Error(`サーバーエラー: ${res.status}`)
    }

    // Persist in sessionStorage (tab-scoped; auto-cleared on tab close)
    sessionStorage.setItem(SESSION_KEY, toBase64(mk))
    setMasterKey(mk)
    setApiToken(tokenHex)
  }, [])

  const logout = useCallback(() => {
    sessionStorage.removeItem(SESSION_KEY)
    setMasterKey(null)
    setApiToken(null)
  }, [])

  const value = useMemo<AuthState>(
    () => ({
      // Check masterKey only: apiToken is derived asynchronously after restore
      // from sessionStorage, so checking both would cause a /login flash on reload.
      // API hooks guard themselves with `enabled: !!apiToken`.
      isAuthenticated: !!masterKey,
      masterKey,
      apiToken,
      login,
      logout,
    }),
    [masterKey, apiToken, login, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}

export function useAuthContext(): AuthState {
  return useContext(AuthContext)
}
