## SoliDrop Web フロントエンド — 設計判断整理

### プロジェクト背景

- **リポジトリ:** `nafell/solidrop`
- **既存システム:** Rust/axum の API サーバー（VPS on XServer）、PC CLI（Rust）
- **単一ユーザー**前提。マルチテナンシーなし
- **API サーバーはコントロールプレーンのみ**。ファイルデータは API サーバーを経由せず、クライアント→S3 直接転送（プレサインド URL 方式）
- **クライアントサイド E2EE**: AES-256-GCM、鍵導出は Argon2id（パスワード→マスター鍵）＋ HKDF-SHA256（マスター鍵→ファイル鍵 / API トークン）

### 暗号化ファイルフォーマット

```
[Header: 45 bytes]
  Magic:    "SOLIDROP\x01" (8 bytes)
  Version:  u8 (1 byte)
  Salt:     [u8; 16] (16 bytes)
  Nonce:    [u8; 12] (12 bytes)
  OrigSize: u64 LE (8 bytes)
[Body]
  AES-256-GCM ciphertext + authentication tag
```

### API エンドポイント（実装済み）

```
GET  /health
POST /api/v1/presign/upload    { path, content_hash, size_bytes } → { url, expires_in }
POST /api/v1/presign/download  { path } → { url, expires_in }
GET  /api/v1/files             → { files: [{path, size_bytes, last_modified, content_hash}], next_token }
DELETE /api/v1/files/{encoded_path}   (未実装)
POST /api/v1/files/move               (未実装)
POST /api/v1/cache/report             (未実装)
```

認証: `Authorization: Bearer <api_token_hex>`（HKDF 導出トークン）

---

### スタック

```
React 19 / TypeScript / Vite / TanStack Query
```

---

### 設計判断が必要な項目

#### 1. Web フロントエンドの位置づけ

**選択肢:**
- A) PC CLI の GUI 代替（CLI と同等機能、デスクトップブラウザ向け）
- B) Flutter アプリの Web 版（iPad Safari も対象）
- C) A ＋ B 両対応（レスポンシブ）

→ 現状 Flutter アプリは未作成。Aのみで良い。

#### 2. ブラウザでの暗号化実装

Rust の `solidrop-crypto` はブラウザでは使えない（FFI 不可）。選択肢:

| 方式 | メリット | デメリット |
|---|---|---|
| **Web Crypto API** (`SubtleCrypto`) | ネイティブ、ゼロ依存、FIPS 準拠 | Argon2id 非対応（別途実装必要） |
| **WebAssembly（solidrop-crypto を wasm-pack でコンパイル）** | ロジック共有、実装差異ゼロ | wasm-pack ビルド追加、バンドルサイズ増 |
| **JS ライブラリ（noble-ciphers 等）** | Argon2id 含め全対応 | 依存追加、監査コスト |

**推奨判断軸:** AES-256-GCM と HKDF は Web Crypto API でネイティブ対応。Argon2id だけ JS ライブラリ（`argon2-browser` 等）を使うハイブリッドが最も実用的。wasm-pack は将来の選択肢として保留。

#### 3. マスター鍵のセッション管理

Argon2id は計算コストが高い（設計意図）。ページ遷移のたびに再計算させるのは UX 上問題。

**選択肢:**
- A) `sessionStorage` に派生済みマスター鍵（CryptoKey オブジェクト or raw bytes）を保持（タブを閉じると消える）
- B) `IndexedDB` の `CryptoKey`（non-extractable）でタブまたいで保持
- C) `sessionStorage` + ロック画面（一定時間操作なしで再入力要求）

→ **セキュリティポリシーの判断が必要。** 個人用途なのでAが最もシンプル。

#### 4. API トークンの保存場所

HKDF から導出した API トークンをどこに置くか。

- セッション中はメモリ（React state / TanStack Query の queryClient cache）に保持
- `localStorage` への永続化は**禁止**（XSS で漏洩）
- マスター鍵が sessionStorage にあれば、API トークンはそこから都度再導出可能

#### 5. 大容量ファイルのアップロード処理

最大 55MB のファイルを扱う。

| 方式 | 実装 | 留意点 |
|---|---|---|
| **シングルチャンク** | `fetch` + presigned PUT | S3 の 5GB 上限内なら問題なし |
| **Multipart Upload** | 複数 presigned URL + `CompleteMultipartUpload` | 55MB ならオーバーエンジニアリング |
| **ストリーミング暗号化** | ReadableStream + AES-GCM | ブラウザサポート限定的、実装複雑 |

**推奨:** シングルチャンクで十分。ファイルを `ArrayBuffer` として読み、暗号化後に `fetch` で PUT。進捗は `XMLHttpRequest` の `progress` イベントで取得（`fetch` は upload progress 未対応）。

#### 6. TanStack Query の使い方

サーバー状態（ファイル一覧等）は TanStack Query で管理。暗号化関連のクライアント状態は別途管理が必要。

```typescript
// ファイル一覧 — TanStack Query が管理
useQuery({ queryKey: ['files'], queryFn: fetchFiles })

// 鍵・認証状態 — TanStack Query の外（React context か Zustand）
const { masterKey, apiToken } = useAuthContext()
```

**判断必要:** クライアント状態管理に何を使うか。Zustand か React Context か。規模的には Context で十分。

#### 7. ルーティング

- **TanStack Router** — TanStack Query と親和性が高い、型安全ルーティング
- **React Router v7** — エコシステム成熟

→ 特に制約がなければ TanStack Router を推奨（同エコシステムで統一）。

#### 8. CORS

API サーバー（Rust/axum）に CORS 設定が必要。現状 `tower-http` の CORS ミドルウェアが使われているが、Web フロントエンドのオリジンを許可リストに追加する必要がある。

```rust
// 追加が必要な設定
CorsLayer::new()
    .allow_origin("https://web.solidrop.nafell.com".parse::<HeaderValue>().unwrap())
    .allow_methods([Method::GET, Method::POST, Method::DELETE])
    .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
```

#### 9. ファイルダウンロード・復号フロー

```
1. GET /api/v1/files → ファイル一覧表示
2. POST /api/v1/presign/download → presigned URL 取得
3. fetch(presigned_url) → 暗号化バイト列
4. ヘッダー検証（magic, version）
5. AES-256-GCM 復号（Web Crypto API）
6. SHA-256 ハッシュ検証
7. Blob URL 生成 → ダウンロードトリガー
```

#### 10. ローカルキャッシュ管理（Cache Report）

iPad 向けの LRU キャッシュ退避機能（`POST /api/v1/cache/report`）は Phase 1 スコープ。Web フロントエンドが PC 向けならこの機能は不要（PC はキャッシュ管理をしない）。iPad Safari 向けも対象にする場合は Origin Private File System (OPFS) を使うことになるが、複雑度が増す。

**推奨:** Phase 1 は PC 向け（アップロード・ダウンロード・一覧）に絞り、キャッシュ管理は除外。

---

### 未解決の判断（セッションで確認すべき事項）

1. Web フロントエンドのスコープ: PC 専用 か iPad 対応も含めるか
2. Argon2id の JS ライブラリ: `argon2-browser` か `@noble/hashes` の Argon2 か、または wasm-pack か
3. クライアント状態管理: React Context で十分か、Zustand を入れるか
4. ルーター: TanStack Router か React Router v7 か
5. フロントエンドの配信場所: VPS の同一ドメインで静的配信か、別サービス（Vercel 等）か（CORS 設計に影響）

---

### 参照すべきファイル（別セッション開始時に読み込むもの）

```
README.md                            # 要件全体（日本語）
docs/design/architecture.md         # アーキテクチャ設計
docs/adr/003-hkdf-derived-api-token.md  # 認証設計の詳細
crates/api-server/SPEC.md           # API エンドポイント定義
crates/crypto/SPEC.md               # 暗号化仕様
```