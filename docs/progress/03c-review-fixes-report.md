# 04 — Code Review Fixes Report

`03-b-code-review-report.md` の全5件の指摘事項に対する修正を実施。

## 修正サマリー

| # | Finding | Severity | 修正内容 | ステータス |
|---|---|---|---|---|
| 1 | `delete.rs` S3エラー誤分類 | High | `SdkError` のHTTPステータスを判別し404のみ `NotFound`、他は `Internal` | 完了 |
| 2 | `file_move.rs` copy_source未エンコード | High | `percent-encoding` クレートでキーをURLエンコード | 完了 |
| 3 | `file_move.rs` 削除失敗時ログ不足 | Medium | `tracing::error!` でコピー済み状態を明示ログ + SPEC.md にbest-effort move仕様を明文化 | 完了 |
| 4 | `cache.rs` last_used文字列ソート | Medium | `chrono::DateTime<Utc>` にパースしてソート、不正値は400 | 完了 |
| 5 | テスト不足 | Medium | 異常系テスト2件（非ignore）+ S3統合テスト2件（ignore）追加 | 完了 |

## 変更ファイル

| ファイル | 変更内容 |
|---|---|
| `crates/api-server/Cargo.toml` | `percent-encoding = "2"`, `chrono` 追加 |
| `crates/api-server/src/routes/delete.rs` | `is_not_found()` ヘルパー追加、S3エラー分類 |
| `crates/api-server/src/routes/file_move.rs` | `S3_KEY_ENCODE_SET` + `utf8_percent_encode` でcopy_sourceエンコード、削除失敗ログ強化 |
| `crates/api-server/src/routes/cache.rs` | `DateTime<Utc>` パース、`ParsedEntry` 内部構造体、400エラー返却 |
| `crates/api-server/tests/api_test.rs` | `test_cache_report_invalid_timestamp`, `test_cache_report_timezone_handling`, `test_move_encoded_key` (ignore), `test_delete_s3_error_not_masked` (ignore) |
| `crates/api-server/SPEC.md` | "Best-Effort Move" セクション追加 |

## 検証結果

- `cargo build` — コンパイル成功
- `cargo test -p solidrop-api-server` — 9 passed, 0 failed, 8 ignored
- `cargo clippy --all-targets` — 警告なし（pre-existing dead_code warningのみ）
- `cargo fmt --all -- --check` — フォーマット準拠

## 追加テスト詳細

### 非ignoreテスト（S3不要）
- **`test_cache_report_invalid_timestamp`**: 不正な `last_used` 値で400 BAD_REQUESTが返ることを検証
- **`test_cache_report_timezone_handling`**: `+09:00` と `Z` 表記が混在する場合に正しくUTC変換・ソートされることを検証

### ignoreテスト（MinIO統合）
- **`test_move_encoded_key`**: スペース入りキー (`my drawing (1).enc`) の移動が成功することを検証
- **`test_delete_s3_error_not_masked`**: S3接続不可時に404ではなく500が返ることを検証
