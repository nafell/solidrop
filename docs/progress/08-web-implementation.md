# Phase 1 Web フロントエンド 実装レポート

**日付:** 2026-03-11
**対応ブランチ:** feat/webui
**ステータス:** Phase 1 実装完了・ビルド確認済み

---

## 概要

`docs/progress/07-web-design.md` で設計した Phase 1 スコープ（アップロード・ダウンロード・一覧表示）を `web/` ディレクトリに実装した。
`tsc --noEmit` と `vite build` が共にエラーなしで通過している。

---

## 実装したファイル一覧

### プロジェクト設定

| ファイル | 内容 |
|---|---|
| `web/package.json` | React 19, TanStack Router/Query v5, @noble/hashes 1.x |
| `web/tsconfig.json` | ES2022, bundler moduleResolution, `"types": ["vite/client"]` |
| `web/vite.config.ts` | React plugin + dev proxy (`/api`, `/health` → localhost:3000) |
| `web/index.html` | SPA エントリーポイント |
| `web/nginx.conf` | SPA フォールバック + 静的アセットキャッシュ |
| `web/Dockerfile` | node:22 マルチステージビルド → nginx:1.27-alpine |

### 暗号化レイヤー (`src/crypto/`)

| ファイル | 役割 |
|---|---|
| `format.ts` | SoliDrop 45 バイトヘッダーの parse / build。Rust の `solidrop-crypto` と byte-identical |
| `argon2.ts` | `@noble/hashes/argon2` で Argon2id 鍵導出（t=2, m=19456, p=1）。`VITE_MASTER_SALT_HEX` を読む |
| `hkdf.ts` | Web Crypto API (SubtleCrypto) で HKDF-SHA256。`deriveApiToken` と `deriveFileKey` |
| `aes.ts` | Web Crypto API で AES-256-GCM 暗号化・復号・SHA-256 ハッシュ |
| `index.ts` | 全 crypto 関数の re-export |

### API レイヤー (`src/api/`)

| ファイル | 役割 |
|---|---|
| `types.ts` | `FileEntry`, `PresignUploadRequest`, `FilesResponse` 等の型定義 |
| `client.ts` | `Authorization: Bearer` ヘッダーを付与する `apiFetch` ラッパー、`ApiRequestError` |
| `presign.ts` | presign/upload・presign/download + XHR ベースの `putToS3`（upload progress 対応） |
| `files.ts` | `fetchFiles`、`deleteFile` |

### 状態管理・フック

| ファイル | 役割 |
|---|---|
| `context/AuthContext.tsx` | Argon2id ログイン・sessionStorage 永続化・HKDF トークン導出・ログアウト |
| `hooks/useFiles.ts` | TanStack Query でファイル一覧取得 |
| `hooks/useUpload.ts` | 暗号化 → presign → S3 PUT → キャッシュ invalidate |
| `hooks/useDownload.ts` | presign → S3 GET → 復号 → Blob URL ダウンロード |

### ルーティング・UI

| ファイル | 役割 |
|---|---|
| `router.tsx` | TanStack Router 手動ルートツリー (`/`, `/login`, `/files`)、auth guard |
| `App.tsx` | `QueryClientProvider` + `AuthProvider` + `RouterProvider` の三層ラップ |
| `pages/LoginPage.tsx` | `/login` ページ |
| `pages/FilesPage.tsx` | `/files` ページ（ヘッダー + ファイル一覧 + アップロード） |
| `components/LoginForm.tsx` | パスワード入力フォーム |
| `components/FileList.tsx` | ファイル一覧テーブル（名前・サイズ・日時・DL ボタン） |
| `components/UploadButton.tsx` | ファイル選択 + 進捗バー |
| `components/DownloadButton.tsx` | 復号付きダウンロードボタン |
| `src/index.css` | CSS カスタムプロパティベースのダークテーマ |

---

## 設計書からの変更点・実装時に確定した判断

### 1. 認証確認エンドポイントの変更

設計書では `/health` を使うと記載していたが、`/health` は認証不要なため誤ったパスワードでも常に 200 を返す。
→ `GET /api/v1/files?limit=1` に変更（認証が必要なエンドポイント）。

### 2. `isAuthenticated` の判定基準

設計書の実装例では `!!masterKey && !!apiToken` としていたが、`apiToken` は masterKey から非同期（HKDF）で導出されるため、ページリロード時に masterKey は sessionStorage から即時復元されるのに対し apiToken は useEffect での非同期導出を待つ必要がある。この非対称性によりリロード直後に `/login` へ一瞬リダイレクトするフラッシュが発生する。
→ `isAuthenticated: !!masterKey` に変更。API フックは `enabled: !!apiToken` で自己ガード済み。

詳細: **ADR-010** 参照。

### 3. TypeScript 5.7 の `Uint8Array` 型問題

TypeScript 5.7 では DOM 型が強化され、`Uint8Array<ArrayBuffer>` と `Uint8Array<ArrayBufferLike>` が区別される。関数パラメーターや `.slice()` は `Uint8Array<ArrayBufferLike>` を返すが、Web Crypto API は `Uint8Array<ArrayBuffer>` を要求する。

**対応策:**
- 小サイズバッファ（salt 16B、nonce 12B、masterKey 32B）: `new Uint8Array(arr)` でコピー → 新規 `ArrayBuffer` が確保され `Uint8Array<ArrayBuffer>` になる
- 大サイズバッファ（plaintext 最大 55MB、body）: `as unknown as Uint8Array<ArrayBuffer>` キャスト（コピーコスト回避）
- `tsconfig.json` に `"types": ["vite/client"]` を追加（`import.meta.env` 型解決）

### 4. TanStack Router: 手動ルートツリー採用

ファイルベースルーティング（`@tanstack/router-plugin/vite`）はコード生成（`routeTree.gen.ts`）が必要でビルドパイプラインが複雑になる。Phase 1 のルートは 3 画面のみなので手動 `createRoute` + `addChildren` で構成。

---

## ビルド成果物

```
dist/index.html                   0.39 kB │ gzip:   0.27 kB
dist/assets/index-*.css           3.15 kB │ gzip:   1.09 kB
dist/assets/index-*.js          345.64 kB │ gzip: 109.85 kB
```

バンドルの大部分は TanStack Router/Query と @noble/hashes（Argon2id）。

---

## 初回動作確認で発見・修正したバグ (2026-03-11)

### BUG-1: `VITE_MASTER_SALT_HEX` 未設定

**症状:** ログイン画面で「VITE_MASTER_SALT_HEX is not configured or invalid」エラー。
**原因:** `web/.env.local` が存在せず、ビルド時環境変数が未注入だった。
**修正:** `~/.config/solidrop/master.salt` の hex 値を `web/.env.local` に記載。

```
VITE_MASTER_SALT_HEX=<master.saltの16進数>
```

### BUG-2: Vite dev server プロキシのポート番号ずれ

**症状:** `GET /api/v1/files?limit=1` が 404 (ログイン失敗)。
**原因:** `vite.config.ts` のプロキシが `localhost:3000` を向いていたが、ローカルの API staging コンテナは `localhost:3001` で動作していた。
**修正:** `vite.config.ts` のプロキシを `localhost:3001` に変更。

### BUG-3: `FileEntry.key` → `path` フィールド名の不一致

**症状:** ファイル一覧表示後に「TypeError: can't access property 'split', file.key is undefined」。
**原因:** `src/api/types.ts` の `FileEntry` に `key: string` と定義していたが、API サーバー (`crates/api-server/src/routes/files.rs`) の実際のレスポンスフィールド名は `path`。
**修正:** `types.ts` の `FileEntry.key` → `path` に変更。`FileList.tsx` の参照箇所も同様に変更。

### BUG-4: `PresignUploadResponse.upload_url` / `PresignDownloadResponse.download_url` の不一致

**症状:** アップロード時に `/undefined` へ PUT が発生（404）。
**原因:** `types.ts` のレスポンス型を `upload_url`・`download_url` と定義していたが、API の実際のフィールド名は `url`（アップロード・ダウンロード共通）。
**修正:**
- `types.ts`: `PresignUploadResponse.upload_url` → `url`、`PresignDownloadResponse.download_url` → `url`
- `useUpload.ts`: `{ upload_url }` → `{ url }`
- `useDownload.ts`: `{ download_url }` → `{ url: downloadUrl }`

### BUG-5: S3 CORS 設定の欠如

**症状:** ブラウザから S3 へのアップロード時に「CORS header 'Access-Control-Allow-Origin' missing」。
**原因:** `infra/terraform/s3.tf` に CORS 設定が存在しなかった。プレサインド PUT は `x-amz-meta-content-hash` カスタムヘッダーを含むため、ブラウザが CORS プリフライトを送信するが S3 が拒否した。
**修正:** `s3.tf` に `aws_s3_bucket_cors_configuration` リソースを追加し `terraform apply` で適用。

```hcl
cors_rule {
  allowed_headers = ["*"]
  allowed_methods = ["GET", "PUT", "HEAD"]
  allowed_origins = ["http://localhost:5173", "http://127.0.0.1:5173",
                     "https://staging.web.solidrop.nafell.dev", "https://web.solidrop.nafell.dev"]
  expose_headers  = ["ETag"]
  max_age_seconds = 3600
}
```

あわせて `variables.tf` の `bucket_name` デフォルト値が `nafell-solidrop-storage` と誤っていた（実際は `nafell-solidrop-staging`）ため修正。

### BUG-6: S3 PUT 時に `x-amz-meta-content-hash` ヘッダーが欠落

**症状:** CORS 解消後も S3 PUT が 403 Forbidden。
**原因:** プレサインド URL の `X-Amz-SignedHeaders` に `x-amz-meta-content-hash` が含まれており、実際の PUT リクエストにもこのヘッダーが必須だった。`putToS3` がカスタムヘッダーを送っていなかった。
**修正:** `presign.ts` の `putToS3` に `contentHash` 引数を追加し、`xhr.setRequestHeader('x-amz-meta-content-hash', contentHash)` を設定。`useUpload.ts` から `contentHash` を渡すよう変更。

---

## 残課題・Phase 2 スコープ

| 機能 | 備考 |
|---|---|
| ファイル削除 | `DELETE /api/v1/files/*path` 実装済み、UI 未実装 |
| ファイル移動 | `POST /api/v1/files/move`、UI 未実装 |
| ページネーション | `next_token` による無限スクロール |
| アップロード時のパス選択 | 現状はファイル名をそのまま path に使用。フォルダ構造指定 UI が必要 |
| Web Worker での Argon2id 実行 | 現状はメインスレッドで実行（約 1–2 秒のブロック）。ログイン時の UX 改善余地あり |
| Argon2id ソルトの動的取得 | 現状は `VITE_MASTER_SALT_HEX` でビルド時注入。API エンドポイント経由の動的取得は Phase 2 で検討（ADR-008） |

---

## 関連 ADR

| ADR | 内容 |
|---|---|
| ADR-007 | Web Crypto API + @noble/hashes ハイブリッド暗号化実装 |
| ADR-008 | Argon2id ソルトのビルド時注入（VITE_MASTER_SALT_HEX） |
| ADR-009 | Caddy 同一オリジンプロキシ（CORS 不要設計） |
| ADR-010 | sessionStorage による masterKey セッション保持 |
