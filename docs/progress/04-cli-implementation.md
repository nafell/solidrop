# 04 — PC CLI 実装レポート

> Phase 1 CLI (`solidrop` バイナリ) の全コマンド実装を完了。コードレビュー後の修正も含む。

コミット範囲:
- `b913d64` — CLI全コマンド実装（api_client / master_key / 6コマンド / API契約テスト）
- レビュー修正（`04c-cli-review-fixes-report.md` 参照）— URL安全性 / sync相対パス / upload_dir削除 / テストリネーム / env競合解消

---

## 実装内容

### API クライアント (`src/api_client.rs`)

`reqwest::Client` をラップし、全 API エンドポイントへの型付きメソッドを提供する。

| メソッド | HTTP | パス | 用途 |
|---|---|---|---|
| `presign_upload` | POST | `/presign/upload` | S3 PUT 署名URL取得 |
| `presign_download` | POST | `/presign/download` | S3 GET 署名URL取得 |
| `list_files` | GET | `/files` | ファイル一覧（ページネーション対応） |
| `delete_file` | DELETE | `/files/{path}` | リモートファイル削除 |
| `move_file` | POST | `/files/move` | リモートファイル移動 |
| `put_to_s3` | PUT | 署名URL | 暗号文をS3に直接アップロード |
| `get_from_s3` | GET | 署名URL | 暗号文をS3から直接ダウンロード |

**設計上の注意点:**
- `base_url` は config の `server.endpoint`（`/api/v1` を含む）をそのまま使用
- API キーは `server.api_key_env` で指定した環境変数名から実行時に読み取る（設定ファイルにキー本体を書かない設計）
- 非 2xx レスポンスはボディの `{"error": {"code", "message"}}` をパースして anyhow エラーに変換
- `delete_file` は `percent_encoding` で各パスセグメントを URL エンコード（`/` はセグメント区切りとして維持）

### マスターキー取得 (`src/master_key.rs`)

MVP 実装は環境変数 `SOLIDROP_MASTER_KEY`（hex エンコード 64 文字 = 32 バイト）から取得。

```
未設定     → anyhow エラー（openssl rand -hex 32 の使い方ヒント付き）
不正 hex   → anyhow エラー（"not valid hex" メッセージ）
長さ不正   → anyhow エラー（実際のバイト数を表示）
```

内部のバリデーションロジックは `parse_master_key_hex(&str)` として pure function に分離してあり、テストが環境変数操作なしで実行可能。

OS keychain（`keyring` crate）対応は将来追加予定。config の `keychain_service` / `keychain_account` フィールドはその際に使用する。

### 6つのコマンド実装

#### `upload`

```
solidrop upload <file_path>
```

フロー: ファイル読み込み → SHA-256 ハッシュ計算（平文） → AES-256-GCM 暗号化 → リモートパス生成 → presign_upload → S3 PUT

- リモートパス形式: `active/{YYYY-MM}/{filename}.enc`
- `size_bytes` は暗号文のサイズ（ヘッダー + 暗号文 + GCM タグ）
- 出力: `Uploaded: {local} -> {remote} ({bytes} bytes)`

#### `download`

```
solidrop download <remote_path>
```

フロー: presign_download → S3 GET → AES-256-GCM 復号 → basename から `.enc` 除去 → `download_dir` に保存

- `download_dir` がなければ `create_dir_all` で自動作成
- 出力: `Downloaded: {remote} -> {local}`
- 注: content_hash 検証は未実装（AES-GCM 認証タグが改ざん検知を担保）

#### `list`

```
solidrop list [--prefix <prefix>]
```

フロー: `list_files` をページネーションでループ（`limit=100`） → 全ファイルを収集後に表示

出力フォーマット:
```
  30.0 MB  2026-02-10T15:30:00Z  active/2026-02/illustration-01.clip.enc
   5.2 MB  2026-02-11T10:00:00Z  transfer/2026-02-11/reference.png.enc

2 file(s)
```

`format_size()` で自動的に B / KB / MB / GB を切り替え（ユニットテスト4件）。

#### `sync`

```
solidrop sync
```

フロー: `transfer/` プレフィックスのファイル一覧を全取得 → ローカル存在確認 → 未取得ファイルを presign → GET → 復号 → 保存

**コードレビュー後に修正**: `transfer/` 以降の相対パス構造を `download_dir` 配下に維持する。

例: `transfer/2026-02-11/reference.png.enc` → `{download_dir}/2026-02-11/reference.png`

これにより `transfer/a/report.enc` と `transfer/b/report.enc` が同一 basename でも衝突しない。

出力: `Sync complete: {n} downloaded, {m} skipped`

#### `delete`

```
solidrop delete <remote_path>
```

`delete_file(remote_path)` を呼ぶだけ。出力: `Deleted: {remote_path}`

#### `move`

```
solidrop move <from> <to>
```

`move_file(from, to)` を呼ぶだけ。出力: `Moved: {from} -> {to}`

モジュール名は `move_cmd`（`move` は Rust の予約語）。

---

## 設定ファイル (`src/config.rs`)

```toml
[server]
endpoint = "https://your-vps-domain.com/api/v1"
api_key_env = "SOLIDROP_API_KEY"

[storage]
download_dir = "~/Art/synced"

[crypto]
keychain_service = "solidrop"
keychain_account = "master-key"
```

設定ファイルのパスは `directories` crate で解決:
- Linux: `~/.config/solidrop/config.toml`
- macOS: `~/Library/Application Support/dev.nafell.solidrop/config.toml`

`upload_dir` は当初 `StorageConfig` に定義していたが、upload コマンドはファイルパスを引数で直接受け取る設計のため不要と判断し削除（コードレビュー後）。

---

## テスト

### ユニットテスト（8件、`cargo test -p solidrop-cli` で実行）

| テスト | ファイル | 内容 |
|---|---|---|
| `test_format_size_bytes` | `list.rs` | 0〜1023 B の表示 |
| `test_format_size_kilobytes` | `list.rs` | 1024〜1MB 未満 |
| `test_format_size_megabytes` | `list.rs` | 1MB〜1GB 未満 |
| `test_format_size_gigabytes` | `list.rs` | 1GB 以上 |
| `test_valid_master_key` | `master_key.rs` | 正常な 64 文字 hex |
| `test_missing_env_var` | `master_key.rs` | 環境変数未設定 |
| `test_invalid_hex` | `master_key.rs` | 不正 hex 文字列 |
| `test_wrong_length` | `master_key.rs` | 2 バイト（32 バイト不足） |

### API 契約テスト（4件、`#[ignore]`）

`tests/api_contract_test.rs` — docker-compose 環境（MinIO + API サーバー）に対して実行する。

```bash
docker compose up -d
cargo test -p solidrop-cli -- --ignored
```

| テスト | 検証内容 |
|---|---|
| `test_upload_and_list_roundtrip` | upload → list で確認 → delete で cleanup |
| `test_upload_and_download_roundtrip` | upload → download → 平文一致検証 |
| `test_delete` | upload → delete → list で消えたことを確認 |
| `test_move` | upload → move → list / 復号で整合性確認 |

**注**: これらのテストは `reqwest` で API を直接叩く API 契約テスト。`solidrop` バイナリ経由の E2E テスト（`assert_cmd` 使用）は未実装（TODO）。

---

## コードレビュー後の変更

`04b-cli-code-review-report-b913d64.md` の5件の指摘全てを修正（詳細: `04c-cli-review-fixes-report.md`）。

| Finding | 修正概要 |
|---|---|
| URL 構築の安全性（High） | `delete_file` に `percent_encoding` を適用、テスト側は `.query()` ビルダーに変更 |
| sync の basename 衝突（Medium） | `transfer/` 相対パス構造を `download_dir` 配下に維持 |
| `upload_dir` 未使用（Medium） | `StorageConfig` から削除、`upload::run` のシグネチャ整理 |
| テスト名と実態の乖離（Medium） | `cli_integration_test.rs` → `api_contract_test.rs` にリネーム |
| env 競合（Low） | `parse_master_key_hex` を pure function として分離 |

---

## 現状と既知の制限

| 項目 | 状態 |
|---|---|
| 全6コマンド | 完成 |
| API 契約テスト | 完成（docker-compose 必要） |
| CLI E2E テスト | 未実装（`assert_cmd` で将来対応） |
| content_hash 検証（download） | 未実装（AES-GCM タグが代替） |
| OS keychain 統合 | 未実装（env var で代替） |
| 大容量ファイルのストリーミング | 未実装（全バイトを一括メモリロード） |

---

## 次のステップ

- **Phase 1 完了**: API サーバー + PC CLI のセット実装済み。Flutter iPad アプリが次フェーズ。
- **インフラ整備**: 実運用には S3 バケット + AWS IAM + VPS デプロイが必要（`CLAUDE.md` 参照）。
- **E2E テスト**: `assert_cmd` で `solidrop` バイナリを起動するテスト追加。
