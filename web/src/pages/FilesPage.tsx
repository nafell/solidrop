import AppHeader from '../components/AppHeader'
import FileList from '../components/FileList'
import UploadButton from '../components/UploadButton'

export default function FilesPage() {
  return (
    <>
      <AppHeader />
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
