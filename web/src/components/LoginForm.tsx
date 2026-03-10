import { useState, type FormEvent } from 'react'
import { useAuthContext } from '../context/AuthContext'
import { useRouter } from '@tanstack/react-router'

export default function LoginForm() {
  const { login } = useAuthContext()
  const router = useRouter()
  const [password, setPassword] = useState('')
  const [loading, setLoading] = useState(false)
  const [error, setError] = useState<string | null>(null)

  async function handleSubmit(e: FormEvent) {
    e.preventDefault()
    if (!password) return
    setLoading(true)
    setError(null)
    try {
      await login(password)
      router.navigate({ to: '/files' })
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err))
    } finally {
      setLoading(false)
    }
  }

  return (
    <div className="login-wrapper">
      <div className="login-card">
        <h1>SoliDrop</h1>
        <form onSubmit={handleSubmit}>
          <div className="form-group">
            <label htmlFor="password">マスターパスワード</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={e => setPassword(e.target.value)}
              placeholder="パスワードを入力"
              autoFocus
              disabled={loading}
            />
          </div>
          <button type="submit" className="btn-primary" disabled={loading || !password}>
            {loading ? '認証中...' : 'ログイン'}
          </button>
          {error && <p className="error-msg">{error}</p>}
        </form>
      </div>
    </div>
  )
}
