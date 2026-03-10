import { Link, useRouter } from '@tanstack/react-router'
import { useAuthContext } from '../context/AuthContext'
import FileList from '../components/FileList'
import UploadButton from '../components/UploadButton'

export default function FilesPage() {
  const { logout } = useAuthContext()
  const router = useRouter()

  function handleLogout() {
    logout()
    router.navigate({ to: '/login' })
  }

  return (
    <>
      <header className="app-header">
        <h1>SoliDrop</h1>
        <nav className="app-nav">
          <Link to="/files" activeProps={{ className: 'active' }}>一覧</Link>
          <Link to="/viewer" activeProps={{ className: 'active' }}>ビューア</Link>
        </nav>
        <button className="btn-ghost" onClick={handleLogout}>ログアウト</button>
      </header>
      <main className="page">
        <div className="toolbar">
          <h2>ファイル一覧</h2>
          <UploadButton />
        </div>
        <FileList />
      </main>
    </>
  )
}
