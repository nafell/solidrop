# 02 — TBD Decisions Report

> Phase 1開始前に決定が必要だったTBD項目（TBD-1, 2, 3, 5）を決定し、README.md および関連ファイルに反映した。

## 決定事項

| ID | 項目 | 決定内容 | 根拠 |
|---|---|---|---|
| TBD-1 | S3バケット名 | `nafell-solidrop-storage` | ユーザー名 + プロジェクト名で一意性を確保。AWSのグローバル一意性要件を満たす命名 |
| TBD-2 | TLS証明書の取得方法 | Caddy（自動TLS） | ドメイン既存。CaddyはLet's Encryptの自動取得・更新をビルトインで提供し、certbot + nginx構成より運用が簡単 |
| TBD-3 | APIキーの生成・管理方法 | `openssl rand -hex 32`で生成、`API_KEY`環境変数で管理 | 単一ユーザーなので固定APIキーで十分。256bit（64文字hex）でブルートフォース耐性確保 |
| TBD-5 | Argon2idパラメータ | ライブラリデフォルト維持（暫定） | `argon2` crateのデフォルト値はOWASP推奨に準拠。Flutter実装時にiPad実機で実測し、UXに問題があれば調整 |

## 先送り事項

| ID | 項目 | 理由 |
|---|---|---|
| TBD-4 | IAMアクセスキーのローテーション頻度 | Phase 2以降。本番デプロイ時に決定 |
| TBD-6 | BGTaskSchedulerの実装方式 | Flutter実装時に決定 |
| TBD-7 | Flutter側のS3アップロード実装方式 | Flutter実装時に決定 |
| TBD-8 | Flutter暗号化: Dart実装 or Rust FFI | Flutter実装時にパフォーマンス検証後に判断 |

## 変更したファイル

| ファイル | 変更内容 |
|---|---|
| `README.md` | セクション18.1のTBDテーブルでTBD-1, 2, 3, 5のステータスを更新 |
| `infra/terraform/variables.tf` | `bucket_name`変数にデフォルト値`nafell-solidrop-storage`を設定 |
| `.env.example` | 本番バケット名をコメントに追記 |
| `docker-compose.yml` | 変更なし（開発用`solidrop-dev`を維持。本番は`.env`で上書き） |
