# SoliDrop Web フロントエンド — 設計書

**作成日:** 2026-03-10
**ステータス:** Draft
**対象フェーズ:** Phase 1 (PC ブラウザ向け基本機能)

---

## 1. 概要

### スコープ

**PC ブラウザ専用（デスクトップ向け）。** モバイル・iPad Safari は対象外（Flutter アプリが担う）。
Phase 1 スコープ: **アップロード・ダウンロード・一覧表示**。キャッシュ管理・ファイル移動は Phase 2 以降。

### スタック

| 区分 | 技術 |
|---|---|
| UI フレームワーク | React 19 + TypeScript |
| ビルドツール | Vite |
| ルーター | TanStack Router |
| サーバー状態管理 | TanStack Query v5 |
| クライアント状態管理 | React Context |
| 暗号化 (Argon2id) | `@noble/hashes` |
| 暗号化 (AES-GCM / HKDF) | Web Crypto API (SubtleCrypto) |
| スタイリング | TBD (CSS Modules or Tailwind) |

### 配信構成

```
[ユーザー (PCブラウザ)]
        │  HTTPS
        ▼
[Caddy (VPS)]
  staging.web.solidrop.nafell.dev
  web.solidrop.nafell.dev
        │
        ├── /api/*  ──→ [API コンテナ :3001/:3002]
        ├── /health ──→ [API コンテナ :3001/:3002]
        └── /*      ──→ [nginx コンテナ :3011/:3012]
                               │
                         Vite ビルド成果物 (dist/)
```

| 環境 | ドメイン | API ポート | Web ポート |
|---|---|---|---|
| staging | `staging.web.solidrop.nafell.dev` | 3001 | 3011 |
| prod    | `web.solidrop.nafell.dev`         | 3002 | 3012 |

Caddy が `/api/*` を API コンテナに透過することでブラウザから見ると **同一オリジン** になるため、CORS 設定は不要。

---

## 2. ディレクトリ構成

```
web/
├── index.html
├── vite.config.ts
├── tsconfig.json
├── package.json
└── src/
    ├── main.tsx                    # Vite エントリーポイント
    ├── App.tsx                     # ルーター・プロバイダー設定
    ├── crypto/
    │   ├── argon2.ts               # @noble/hashes で Argon2id 鍵導出
    │   ├── hkdf.ts                 # Web Crypto API で HKDF-SHA256
    │   ├── aes.ts                  # Web Crypto API で AES-256-GCM
    │   ├── format.ts               # SoliDrop ヘッダー読み書き
    │   └── index.ts                # re-export
    ├── api/
    │   ├── client.ts               # fetch ラッパー (Authorization ヘッダー付与)
    │   ├── presign.ts              # presign/upload, presign/download
    │   ├── files.ts                # GET /api/v1/files, DELETE, move
    │   └── types.ts                # API レスポンス型定義
    ├── context/
    │   └── AuthContext.tsx         # masterKey / apiToken / セッション管理
    ├── hooks/
    │   ├── useFiles.ts             # TanStack Query: ファイル一覧
    │   ├── useUpload.ts            # アップロードフロー
    │   └── useDownload.ts          # ダウンロードフロー
    ├── routes/
    │   ├── __root.tsx              # TanStack Router ルートレイアウト
    │   ├── index.tsx               # / → /login リダイレクト
    │   ├── login.tsx               # /login パスワード入力画面
    │   └── files/
    │       ├── index.tsx           # /files ファイル一覧
    │       └── $path.tsx           # /files/$path ファイル詳細 (Phase 2)
    └── components/
        ├── LoginForm.tsx
        ├── FileList.tsx
        ├── UploadButton.tsx
        └── DownloadButton.tsx
```

---

## 3. 暗号化実装

### Web Crypto API と @noble/hashes の役割分担

| 操作 | 実装 | 理由 |
|---|---|---|
| Argon2id 鍵導出 | `@noble/hashes` | Web Crypto API は Argon2id 非対応 |
| HKDF-SHA256 | Web Crypto API (SubtleCrypto) | ネイティブ対応、ゼロ追加依存 |
| AES-256-GCM 暗号化/復号 | Web Crypto API (SubtleCrypto) | ネイティブ対応、HW アクセラレーション |
| SHA-256 ハッシュ | Web Crypto API (SubtleCrypto) | ネイティブ対応 |

**wasm-pack（solidrop-crypto の WASM 化）は将来の選択肢として保留。** 現時点ではビルドチェーン追加コストに見合わない。

### SoliDrop ファイルフォーマット (`crypto/format.ts`)

Rust の `solidrop-crypto` が定義するフォーマットと完全互換を保つ:

```
Offset  Size  Field
0       8     Magic: "SOLIDROP\x01" (0x53 4F 4C 49 44 52 4F 50 01)
8       1     Version: 0x01
9       16    Salt (Argon2id / HKDF 用、ファイルごとランダム)
25      12    Nonce (AES-256-GCM 用、ファイルごとランダム)
37      8     Original size (u64 little-endian)
45      ...   AES-256-GCM ciphertext + 16-byte auth tag
```

合計ヘッダー: **45 バイト**。

```typescript
// crypto/format.ts
const MAGIC = new Uint8Array([0x53,0x4F,0x4C,0x49,0x44,0x52,0x4F,0x50,0x01])
const VERSION = 0x01
const HEADER_SIZE = 45

interface SoliDropHeader {
  version: number
  salt: Uint8Array    // 16 bytes
  nonce: Uint8Array   // 12 bytes
  origSize: bigint    // u64
}

function parseHeader(data: Uint8Array): SoliDropHeader
function buildHeader(salt: Uint8Array, nonce: Uint8Array, origSize: number): Uint8Array
```

---

## 4. 鍵導出フロー

ADR-003 の設計を Web で実装する:

```
パスワード (string)
    │
    ▼ Argon2id (@noble/hashes)
    │  params: Argon2::default() 相当
    │  salt: 16 bytes (セッション開始時にサーバーから取得 or 固定 ← TBD)
    ▼
masterKey (32 bytes, raw)
    │
    ├─▶ HKDF-SHA256 (Web Crypto API)
    │     info: "solidrop-api-auth"
    │     salt: なし (zero-length)
    │   ▼
    │   apiToken (32 bytes) → hex string → Authorization: Bearer <hex>
    │
    └─▶ HKDF-SHA256 (Web Crypto API)  ←── ファイルごとに実行
          info: "solidrop-file-encryption"
          salt: per_file_salt (ヘッダーから取得)
        ▼
        fileKey (32 bytes) → CryptoKey (AES-256-GCM 用)
```

> **注意:** Argon2id の salt 管理方針は `crates/crypto/SPEC.md` に準拠する。
> CLI では `derive_master_key(password, salt)` の salt はファイルヘッダーに埋め込まれている。
> Web では認証時の Argon2id salt をどこに保持するか実装時に確定させる（候補: 固定 salt として設定ファイルに持つ、または `sessionStorage`）。

---

## 5. セッション管理

### masterKey の保持

`sessionStorage` に raw bytes（Base64 エンコード）として保持する。

- タブを閉じると自動消去（セキュリティ境界として機能）
- `sessionStorage` は同一タブ内のみアクセス可能
- **localStorage への永続化は禁止**（XSS でのシークレット漏洩リスク）

```typescript
// context/AuthContext.tsx
const SESSION_KEY = 'solidrop_master_key'

function saveMasterKey(masterKey: Uint8Array): void {
  sessionStorage.setItem(SESSION_KEY, btoa(String.fromCharCode(...masterKey)))
}

function loadMasterKey(): Uint8Array | null {
  const stored = sessionStorage.getItem(SESSION_KEY)
  if (!stored) return null
  return new Uint8Array(atob(stored).split('').map(c => c.charCodeAt(0)))
}
```

### apiToken の保持

`apiToken` は masterKey から都度 HKDF で再導出するか、React state / Context に保持する。
**localStorage/sessionStorage への apiToken 保存は禁止**（masterKey があれば再導出可能なため不要）。

---

## 6. 認証フロー

```
1. /login 画面: パスワード入力フォーム表示

2. ユーザーがパスワード送信
   │
   ├─ Argon2id → masterKey  (Web Worker で実行推奨: UI スレッドをブロックしない)
   ├─ HKDF("solidrop-api-auth") → apiToken
   │
   ▼
3. GET /health (Authorization: Bearer <apiToken_hex>)
   │
   ├─ 200 OK → 認証成功
   │   ├─ masterKey を sessionStorage に保存
   │   ├─ AuthContext に masterKey / apiToken をセット
   │   └─ /files へリダイレクト
   │
   └─ 401 → 「パスワードが違います」エラー表示

4. 以降のリクエストはすべて AuthContext の apiToken を使用
```

> **注意:** サーバーは `SHA256(apiToken)` の verifier のみを保持する（ADR-003）。
> クライアントが送信する Bearer トークンは apiToken の hex 文字列（64 chars）。

---

## 7. ルーティング

TanStack Router で型安全なルーティングを実装する:

```
/                   → /login にリダイレクト（未認証時）
/login              → LoginForm コンポーネント（パスワード入力）
/files              → FileList コンポーネント（ファイル一覧）
/files/$path        → ファイル詳細（Phase 2 以降）
```

### 認証ガード

TanStack Router の `beforeLoad` を使い、未認証ユーザーを `/login` にリダイレクト:

```typescript
// routes/__root.tsx
const rootRoute = createRootRoute({
  beforeLoad: ({ context }) => {
    if (!context.auth.isAuthenticated && location.pathname !== '/login') {
      throw redirect({ to: '/login' })
    }
  }
})
```

---

## 8. ファイル一覧

### API 呼び出し

```
GET /api/v1/files?prefix=&limit=100
Authorization: Bearer <apiToken_hex>

Response:
{
  "files": [
    { "key": "path/to/file.clip", "size": 12345678, "last_modified": "2026-03-10T...", "content_hash": "sha256:abc..." }
  ],
  "next_token": null
}
```

バックエンドは Valkey（ADR-006）からメタデータを返す。S3 への直接問い合わせより応答が安定する。

### TanStack Query

```typescript
// hooks/useFiles.ts
export function useFiles(prefix?: string) {
  const { apiToken } = useAuthContext()
  return useQuery({
    queryKey: ['files', prefix],
    queryFn: () => fetchFiles(apiToken, prefix),
    enabled: !!apiToken,
  })
}
```

ページネーションは `next_token` を使う。Phase 1 では limit=100 固定で十分。

---

## 9. アップロードフロー

ADR-002（プレサインド URL）と ADR-001（クライアントサイド暗号化）を組み合わせる:

```
1. ユーザーが File オブジェクトを選択（input[type=file]）

2. File → ArrayBuffer (FileReader or file.arrayBuffer())

3. 暗号化 (crypto/aes.ts)
   ├─ generate random salt (16 bytes)
   ├─ generate random nonce (12 bytes)
   ├─ HKDF(masterKey, salt, "solidrop-file-encryption") → fileKey
   ├─ AES-256-GCM encrypt(fileKey, nonce, plaintext) → ciphertext
   └─ buildHeader(salt, nonce, origSize) + ciphertext → encryptedBytes

4. SHA-256 ハッシュ計算（平文に対して）
   content_hash = "sha256:" + hex(SHA256(plaintext))

5. POST /api/v1/presign/upload
   { path: "uploads/filename.clip", content_hash, size_bytes: encryptedBytes.length }
   → { upload_url: "https://s3.amazonaws.com/..." }

6. PUT <upload_url>  (直接 S3 へ)
   Body: encryptedBytes (Blob)
   ※ fetch の upload progress は未対応 → XMLHttpRequest を使う

7. TanStack Query の ['files'] キャッシュを invalidate
```

### 進捗表示

```typescript
function uploadWithProgress(url: string, data: Blob, onProgress: (pct: number) => void) {
  return new Promise<void>((resolve, reject) => {
    const xhr = new XMLHttpRequest()
    xhr.open('PUT', url)
    xhr.upload.onprogress = (e) => {
      if (e.lengthComputable) onProgress((e.loaded / e.total) * 100)
    }
    xhr.onload = () => xhr.status === 200 ? resolve() : reject(new Error(`${xhr.status}`))
    xhr.onerror = () => reject(new Error('Network error'))
    xhr.send(data)
  })
}
```

---

## 10. ダウンロードフロー

```
1. ユーザーがダウンロードボタンをクリック（path を指定）

2. POST /api/v1/presign/download
   { path: "uploads/filename.clip" }
   → { download_url: "https://s3.amazonaws.com/..." }

3. fetch(download_url) → Response → ArrayBuffer (暗号化済みバイト列)

4. ヘッダー検証 (crypto/format.ts)
   ├─ magic bytes 一致確認 ("SOLIDROP\x01")
   └─ version === 0x01 確認

5. 復号 (crypto/aes.ts)
   ├─ ヘッダーから salt, nonce, origSize を取得
   ├─ HKDF(masterKey, salt, "solidrop-file-encryption") → fileKey
   └─ AES-256-GCM decrypt(fileKey, nonce, ciphertext) → plaintext

6. origSize 検証: plaintext.length === origSize

7. Blob URL 生成 → ダウンロードトリガー
   const url = URL.createObjectURL(new Blob([plaintext]))
   const a = document.createElement('a')
   a.href = url; a.download = filename; a.click()
   URL.revokeObjectURL(url)
```

---

## 11. コンポーネント設計

### AuthContext

```typescript
// context/AuthContext.tsx
interface AuthState {
  isAuthenticated: boolean
  masterKey: Uint8Array | null
  apiToken: string | null          // hex string
  login: (password: string) => Promise<void>
  logout: () => void
}

export const AuthContext = createContext<AuthState>(...)
export function AuthProvider({ children }: { children: ReactNode })
export function useAuthContext(): AuthState
```

### FileList

```typescript
// components/FileList.tsx
// useFiles() で取得したファイル一覧を表示
// 各行に DownloadButton を配置
// カラム: ファイル名 / サイズ / 更新日時 / ダウンロード
```

### UploadButton

```typescript
// components/UploadButton.tsx
// input[type=file] をラップ
// useUpload() フックを呼び出し
// アップロード進捗バーを表示
// 完了後に queryClient.invalidateQueries(['files'])
```

### DownloadButton

```typescript
// components/DownloadButton.tsx
// path を受け取り、クリックでダウンロード開始
// useDownload() フックを呼び出し
// 復号中はローディング表示
```

---

## 12. Docker / Caddy 設定

### Docker コンテナ構成

フロントエンドは nginx コンテナで静的ファイルを配信する。`web/Dockerfile` は multi-stage ビルド（node build + nginx serve）。

| コンテナ名 | イメージ | ホストポート | 用途 |
|---|---|---|---|
| `solidrop-web-staging` | `solidrop-web:staging` | `127.0.0.1:3011:80` | Web staging |
| `solidrop-web-prod`    | `solidrop-web:prod`    | `127.0.0.1:3012:80` | Web prod |
| `solidrop-api-staging` | `solidrop-api-server:staging` | `127.0.0.1:3001:3000` | API staging |
| `solidrop-api-prod`    | `solidrop-api-server:prod`    | `127.0.0.1:3002:3000` | API prod |

### Caddy 設定（staging）

`infra/vps/Caddyfile.web-staging.snippet`:

```caddyfile
staging.web.solidrop.nafell.dev {
    handle /api/* {
        reverse_proxy http://localhost:3001
    }
    handle /health {
        reverse_proxy http://localhost:3001
    }
    handle {
        reverse_proxy http://localhost:3011
    }
}
```

SPA ルーティングのフォールバック（`/path → index.html`）は nginx 側の `try_files` 設定で処理する（`infra/nginx/default.conf`）。

### デプロイ手順

```bash
# PC から VPS へ転送（ローカルビルド）
VPS_HOST=user@<IP> bash infra/vps/deploy.sh staging web

# VPS 上でビルド
bash infra/vps/deploy-inside-vps.sh staging web

# Caddy に web ドメインを追加（初回のみ）
bash infra/vps/caddy-apply.sh staging
```

---

## 13. CORS

Caddy が `/api/*` を API コンテナに透過するため、ブラウザからは **同一オリジン**（`staging.web.solidrop.nafell.dev`）のリクエストとなり、**CORS ヘッダー設定は不要**。

- Web + API アクセス: `https://staging.web.solidrop.nafell.dev/api/v1/...`（same-origin）
- CLI アクセス: `https://staging.api.solidrop.nafell.dev/api/v1/...`（CORS 不要、CLI は Bearer トークンで直接呼ぶ）
- S3 presigned URL: `https://s3.amazonaws.com/...`（クロスオリジンだが署名済み URL は認証不要）

> 開発環境（Vite dev server :5173 ↔ API :3000）では CORS が必要になるため、API サーバーの `CorsLayer` は dev 設定として `localhost:5173` を許可リストに持つ想定。

---

## 14. 実装フェーズ

### Phase 1 スコープ（本設計書の対象）

| 機能 | 画面/コンポーネント |
|---|---|
| パスワード認証（Argon2id → masterKey → apiToken） | `/login`, `AuthContext` |
| ファイル一覧表示 | `/files`, `FileList` |
| ファイルアップロード（暗号化→presigned PUT） | `UploadButton`, `useUpload` |
| ファイルダウンロード（presigned GET→復号→Blob） | `DownloadButton`, `useDownload` |

### Phase 2 以降（スコープ外）

- ファイル削除（`DELETE /api/v1/files/*path`）
- ファイル移動（`POST /api/v1/files/move`）
- ページネーション（`next_token` による無限スクロール）
- ファイル詳細画面（`/files/$path`）
- iPad LRU キャッシュ管理（`POST /api/v1/cache/report`）— Web では不要の可能性

---

## 整合性チェックメモ

### `crates/crypto/SPEC.md` との整合

- [x] ファイルフォーマット (45 バイトヘッダー) — セクション 3 に記載
- [x] Argon2id 鍵導出 → masterKey — セクション 4 に記載
- [x] HKDF-SHA256(masterKey, file_salt, "solidrop-file-encryption") → fileKey — セクション 4 に記載
- [x] AES-256-GCM 暗号化/復号 — セクション 9, 10 に記載
- [x] SHA-256 は**平文**に対して計算 — セクション 9 ステップ 4 に記載

### `crates/api-server/SPEC.md` との整合

- [x] `POST /api/v1/presign/upload` `{ path, content_hash, size_bytes }` → `{ upload_url }` — セクション 9 に記載
- [x] `POST /api/v1/presign/download` `{ path }` → `{ download_url }` — セクション 10 に記載
- [x] `GET /api/v1/files?prefix=&limit=&next_token=` → `{ files, next_token }` — セクション 8 に記載
- [x] `Authorization: Bearer <api_token_hex>` — セクション 6 に記載
- [x] `GET /health` を認証確認に使用 — セクション 6 に記載

### ADR との整合

- [x] ADR-001: クライアントサイド暗号化、鍵はクライアント外に送出しない — セクション 3, 4 に反映
- [x] ADR-002: プレサインド URL でデータが API サーバーを経由しない — セクション 9, 10 に反映
- [x] ADR-003: HKDF 派生 API トークン (`info="solidrop-api-auth"`) — セクション 4, 6 に反映
- [x] ADR-006: Valkey バックエンドによるファイル一覧 API — セクション 8 に反映
- [x] ADR-005: LRU キャッシュ管理は Phase 1 スコープ外 — セクション 14 に明記
