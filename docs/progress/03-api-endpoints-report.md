# 03 — API Server Endpoints Implementation Report

> Phase 0で501スタブだった全6エンドポイントを実装し、Bearer認証ミドルウェアを追加。API Serverが完全に機能する状態になった。

## 実装内容

### 認証ミドルウェア (`src/middleware.rs`)

`Authorization: Bearer <token>` ヘッダーを検証するaxumミドルウェア。`/health`以外の全`/api/v1/*`ルートに適用。

- `from_fn_with_state`パターンで実装（ミドルウェア内で`State<AppState>`を抽出するため）
- トークン不一致・ヘッダー欠損時は`AppError::Unauthorized`（401）を返却
- ルーターは`router_with_auth(state)`で認証付き、`router()`で認証なし（テスト用）の2パターンを提供

### Presigned URL生成 (`src/routes/presign.rs`)

| エンドポイント | メソッド | 機能 |
|---|---|---|
| `/api/v1/presign/upload` | POST | S3 PUTプレサインURL生成 |
| `/api/v1/presign/download` | POST | S3 GETプレサインURL生成 |

- プレサインURL有効期限: 3600秒（1時間）
- アップロード時にS3メタデータ設定: `content-hash`, `original-size`
- Docker環境ではURL書き換え: `S3_ENDPOINT_URL` → `S3_PUBLIC_ENDPOINT_URL`
- レスポンスフィールド名をREADME仕様に合わせて修正: `url` → `upload_url` / `download_url`

### ファイル一覧 (`src/routes/files.rs`)

`GET /api/v1/files?prefix=&limit=&next_token=`

- `list_objects_v2`でS3オブジェクト一覧を取得
- 各オブジェクトに`head_object`を実行し`content-hash`メタデータを取得（N+1パターン、単一ユーザー規模では許容）
- `limit`パラメータは1〜100にクランプ（デフォルト100）
- ページネーション: `next_continuation_token` → `next_token`

### ファイル削除 (`src/routes/delete.rs`)

`DELETE /api/v1/files/*path`

- ワイルドカードパスキャプチャにより、スラッシュを含むS3キーに対応
- `head_object`で存在確認 → 404、`delete_object`で削除 → `{"deleted": true}`

### ファイル移動 (`src/routes/file_move.rs`)

`POST /api/v1/files/move`

- リクエスト: `{ "from": "...", "to": "..." }`
- S3の`copy_object` → `delete_object`パターン（メタデータはコピー時に自動保持）
- active/archivedプレフィックス間の移動を想定

### キャッシュレポート (`src/routes/cache.rs`)

`POST /api/v1/cache/report`

- iPadのローカルファイルリストとストレージ上限を受け取り、LRU方式で削除候補を計算
- 純粋なロジック（S3アクセスなし）— `last_used`の昇順ソートで最古ファイルから候補に追加
- ユニットテスト3件を含む（under limit / over limit / exactly at limit）

### ライブラリエントリポイント (`src/lib.rs`)

統合テストからモジュールをインポート可能にするための`pub mod`再エクスポート。

## 新規ファイル

| ファイル | 内容 |
|---|---|
| `src/lib.rs` | モジュール再エクスポート（テスト用） |
| `src/middleware.rs` | Bearer認証ミドルウェア |
| `src/routes/cache.rs` | キャッシュレポートハンドラー + ユニットテスト3件 |
| `src/routes/delete.rs` | ファイル削除ハンドラー |
| `src/routes/file_move.rs` | ファイル移動ハンドラー |
| `tests/api_test.rs` | 統合テスト13件（非S3: 7件、S3/MinIO: 6件） |

## 変更したファイル

| ファイル | 変更内容 |
|---|---|
| `src/main.rs` | `mod error; mod middleware;`追加、`router_with_auth`使用に変更 |
| `src/routes/mod.rs` | 新モジュール登録、`router()` / `router_with_auth()`の2関数体制に変更 |
| `src/routes/presign.rs` | 501スタブ → 完全実装 |
| `src/routes/files.rs` | 501スタブ → 完全実装 |
| `SPEC.md` | 全コンポーネントのステータスをCompleteに更新、API仕様を実装に合わせて修正 |
| `Cargo.toml` | dev-dependenciesに`tower`、`reqwest`を追加 |

## テスト一覧

### 自動テスト（S3不要）

| テスト名 | 検証内容 |
|---|---|
| `test_health_no_auth` | `/health`が認証なしで200を返す |
| `test_401_without_token` | Authorizationヘッダーなしで401 |
| `test_401_with_wrong_token` | 不正トークンで401 |
| `test_auth_with_valid_token_passes` | 正しいトークンで401以外（S3未接続でも認証は通過） |
| `test_cache_report_no_overage` | ストレージ余裕あり → 空の削除候補 |
| `test_cache_report_with_eviction` | 超過時にLRU順で正しい候補を返す |
| `test_cache_report_empty_files` | 空リスト → 空の削除候補 |

### S3統合テスト（MinIO必須、`#[ignore]`）

| テスト名 | 検証内容 |
|---|---|
| `test_presign_upload_returns_url` | アップロード用プレサインURLの生成 |
| `test_presign_download_returns_url` | ダウンロード用プレサインURLの生成 |
| `test_list_files_empty` | 存在しないプレフィックスで空リスト |
| `test_upload_then_list` | プレサインURLでアップロード → 一覧に表示される |
| `test_delete_nonexistent_returns_404` | 存在しないファイル削除で404 |
| `test_move_file` | ファイル移動（コピー+削除）の正常動作 |

## ローカル環境でのテスト手順

### 1. S3不要テストの実行

MinIO不要。ビルドとテストのみ:

```bash
# ビルド確認
cargo build -p solidrop-api-server

# S3不要テスト（7件）+ ユニットテスト（5件）を実行
cargo test -p solidrop-api-server

# clippy + フォーマットチェック
cargo clippy -p solidrop-api-server --all-targets
cargo fmt -p solidrop-api-server -- --check
```

期待結果: 12 passed, 6 ignored

### 2. S3統合テストの実行（MinIO使用）

MinIOを起動してから、`--ignored`フラグで統合テストを実行:

```bash
# MinIOを起動（初回はイメージダウンロードあり）
docker compose up -d minio minio-init

# MinIOの起動を待つ（minio-initがバケット作成完了するまで）
docker compose logs -f minio-init
# "Bucket solidrop-dev created successfully" が表示されたらCtrl+C

# S3統合テスト実行（環境変数でMinIO接続先を指定）
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
S3_ENDPOINT_URL=http://localhost:9000 \
S3_PUBLIC_ENDPOINT_URL=http://localhost:9000 \
S3_BUCKET=solidrop-dev \
cargo test -p solidrop-api-server -- --ignored

# 全テスト（非S3 + S3統合）を一括実行
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
S3_ENDPOINT_URL=http://localhost:9000 \
S3_PUBLIC_ENDPOINT_URL=http://localhost:9000 \
S3_BUCKET=solidrop-dev \
cargo test -p solidrop-api-server -- --include-ignored
```

期待結果: 18 passed (ユニット5 + 統合非S3: 7 + 統合S3: 6)

### 3. 手動動作確認（curl）

API Serverを直接起動して手動でエンドポイントを確認する場合:

```bash
# MinIOが起動していることを確認
docker compose up -d minio minio-init

# API Serverをローカル起動
S3_BUCKET=solidrop-dev \
API_KEY=dev-api-key \
AWS_ACCESS_KEY_ID=minioadmin \
AWS_SECRET_ACCESS_KEY=minioadmin \
AWS_REGION=ap-northeast-1 \
S3_ENDPOINT_URL=http://localhost:9000 \
S3_FORCE_PATH_STYLE=true \
S3_PUBLIC_ENDPOINT_URL=http://localhost:9000 \
cargo run -p solidrop-api-server
```

別ターミナルで:

```bash
# ヘルスチェック（認証不要）
curl http://localhost:3000/health
# → {"status":"ok"}

# 認証なしで401確認
curl -s -o /dev/null -w "%{http_code}" \
  -X POST http://localhost:3000/api/v1/presign/upload \
  -H "Content-Type: application/json" \
  -d '{"path":"test.enc","content_hash":"abc","size_bytes":100}'
# → 401

# プレサインURL取得（アップロード）
curl -X POST http://localhost:3000/api/v1/presign/upload \
  -H "Authorization: Bearer dev-api-key" \
  -H "Content-Type: application/json" \
  -d '{"path":"test/hello.enc","content_hash":"abc123","size_bytes":11}'
# → {"upload_url":"http://localhost:9000/solidrop-dev/test/hello.enc?X-Amz-..."}

# プレサインURLでファイルアップロード（上記のupload_urlを使用）
curl -X PUT "<upload_url>" \
  -H "x-amz-meta-content-hash: abc123" \
  -H "x-amz-meta-original-size: 11" \
  -d "hello world"
# → 200 OK

# ファイル一覧
curl http://localhost:3000/api/v1/files?prefix=test/ \
  -H "Authorization: Bearer dev-api-key"
# → {"files":[{"key":"test/hello.enc","size":11,...}],"next_token":null}

# ファイル移動
curl -X POST http://localhost:3000/api/v1/files/move \
  -H "Authorization: Bearer dev-api-key" \
  -H "Content-Type: application/json" \
  -d '{"from":"test/hello.enc","to":"archived/hello.enc"}'
# → {"moved":true}

# ファイル削除
curl -X DELETE http://localhost:3000/api/v1/files/archived/hello.enc \
  -H "Authorization: Bearer dev-api-key"
# → {"deleted":true}

# キャッシュレポート
curl -X POST http://localhost:3000/api/v1/cache/report \
  -H "Authorization: Bearer dev-api-key" \
  -H "Content-Type: application/json" \
  -d '{
    "local_files": [
      {"path":"old.enc","content_hash":"h1","size_bytes":200,"last_used":"2026-01-01T00:00:00Z"},
      {"path":"new.enc","content_hash":"h2","size_bytes":300,"last_used":"2026-02-01T00:00:00Z"}
    ],
    "storage_limit_bytes": 400
  }'
# → {"evict_candidates":[{"path":"old.enc","reason":"lru","last_used":"2026-01-01T00:00:00Z"}]}
```

### 4. テスト後のクリーンアップ

```bash
# MinIOを停止
docker compose down

# MinIOデータも削除する場合
docker compose down -v
```

## 検証結果

- `cargo build -p solidrop-api-server` — 成功
- `cargo test -p solidrop-api-server` — 12 passed, 6 ignored
- `cargo clippy -p solidrop-api-server --all-targets` — クリーン（`router()`のdead_code警告のみ、テスト用に意図的に保持）
- `cargo fmt -p solidrop-api-server -- --check` — クリーン
