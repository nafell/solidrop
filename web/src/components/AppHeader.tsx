import { useState } from 'react'
import { Link, useRouter } from '@tanstack/react-router'
import { useAuthContext } from '../context/AuthContext'
import DebugPanel from './DebugPanel'

export default function AppHeader() {
  const { logout } = useAuthContext()
  const router = useRouter()
  const [debugOpen, setDebugOpen] = useState(false)

  function handleLogout() {
    logout()
    router.navigate({ to: '/login' })
  }

  return (
    <header className="app-header">
      <h1>SoliDrop</h1>
      <nav className="app-nav">
        <Link to="/files" activeProps={{ className: 'active' }}>一覧</Link>
        <Link to="/viewer" activeProps={{ className: 'active' }}>ビューア</Link>
      </nav>
      <div style={{ display: 'flex', gap: 8 }}>
        <button className="btn-ghost" onClick={() => setDebugOpen(o => !o)}>Debug</button>
        <button className="btn-ghost" onClick={handleLogout}>ログアウト</button>
      </div>
      {debugOpen && <DebugPanel onClose={() => setDebugOpen(false)} />}
    </header>
  )
}
