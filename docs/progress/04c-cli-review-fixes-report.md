# 04c — CLI Code Review Fixes Report

`04b-cli-code-review-report-b913d64.md` の全5件の指摘事項に対する修正を実施。

## 修正サマリー

| # | Finding | Severity | 修正内容 | ステータス |
|---|---|---|---|---|
| 1 | パス/クエリ未エンコードのURL組み立て | High | `percent-encoding` クレートで各パスセグメントをエンコード、テスト側は `.query()` ビルダー使用 | 完了 |
| 2 | sync が basename で保存しリモート衝突 | Medium | `transfer/` プレフィックス除去後の相対パス構造を `download_dir` 配下に維持 | 完了 |
| 3 | `upload_dir` 設定が未使用 | Medium | `StorageConfig` から `upload_dir` 削除、`upload::run` から `_config` パラメータ削除 | 完了 |
| 4 | 「CLI統合テスト」がCLIを通していない | Medium | ファイルリネーム `api_contract_test.rs`、doc comment修正、E2E TODO追記 | 完了 |
| 5 | master key テストの環境変数競合 | Low | `parse_master_key_hex` pure function分離、3/4テストからenv操作を排除 | 完了 |

## 変更ファイル

| ファイル | 変更内容 |
|---|---|
| `crates/cli/Cargo.toml` | `percent-encoding = "2"` 追加 |
| `crates/cli/src/api_client.rs` | `PATH_SEGMENT_ENCODE_SET` 定義、`encode_path_segments()` 関数追加、`delete_file` で使用 |
| `crates/cli/src/commands/sync.rs` | `strip_prefix("transfer/")` で相対パス取得、`create_dir_all` でサブディレクトリ自動作成 |
| `crates/cli/src/config.rs` | `StorageConfig` から `upload_dir` フィールド削除 |
| `crates/cli/src/commands/upload.rs` | `_config: &CliConfig` パラメータ削除 |
| `crates/cli/src/main.rs` | upload dispatch から `&config` 引数削除 |
| `crates/cli/src/master_key.rs` | `parse_master_key_hex()` pure function分離、テスト3件をenv非依存に書き換え |
| `crates/cli/tests/cli_integration_test.rs` → `api_contract_test.rs` | リネーム、doc comment修正、`list_files` を `.query()` ベースに、`delete_file` を `percent_encoding` ベースに |

## 各Finding詳細

### Finding 1: URL構築の安全化

**api_client.rs**: `encode_path_segments()` を追加。`/` で分割した各セグメントを `percent_encoding::utf8_percent_encode` でエンコードし、`/` で再結合する。axum の `*path` wildcard はパス全体を1文字列として受け取るため、`/` 自体はエンコードしない。

**api_contract_test.rs**: `list_files` ヘルパーは手組みクエリ文字列 `?prefix={p}` を廃止し、reqwest の `.query(&[("prefix", p)])` に変更。`delete_file` ヘルパーも同様にセグメント単位のパーセントエンコードを適用。

### Finding 2: sync の相対パス保持

変更前: `Path::new(&file.key).file_name()` で basename のみ取得 → 異なるサブディレクトリの同名ファイルが衝突。

変更後: `file.key.strip_prefix("transfer/")` で `transfer/` 以降の相対パスを取得し、`.strip_suffix(".enc")` で暗号化拡張子を除去。`download_dir.join(relative)` で保存パスを構築し、`create_dir_all(parent)` で中間ディレクトリを自動作成。

例: `transfer/2026-02-11/reference.png.enc` → `download_dir/2026-02-11/reference.png`

### Finding 3: `upload_dir` 削除

CLI の upload コマンドはファイルパスを引数で直接受け取る設計のため、`upload_dir` は不要と判断。README の TOML 例にも `upload_dir` の記載はなく、設計意図が不明確だった。SPEC.md のconfig例も併せて更新。

### Finding 4: テストファイルリネーム

既存テストは reqwest でAPIを直接叩いており、CLIバイナリ経由ではない。名前と実態を一致させるため `api_contract_test.rs` にリネームし、doc comment を「API contract tests」に変更。将来の `assert_cmd` E2Eテスト追加のTODOを記載。

### Finding 5: master key テストのenv競合解消

`acquire_master_key()` 内部のhexパース+バリデーションロジックを `parse_master_key_hex(&str) -> Result<[u8; 32]>` として分離。`test_valid_master_key`, `test_invalid_hex`, `test_wrong_length` の3テストは env 操作不要になり、並列実行時の競合が解消。`test_missing_env_var` のみ env を触るが、`remove_var` のみの冪等操作で競合リスクは極めて低い。

## 検証結果

- `cargo build -p solidrop-cli` — コンパイル成功
- `cargo test -p solidrop-cli` — 8 passed, 0 failed, 4 ignored
- `cargo clippy -p solidrop-cli --all-targets` — 新規警告なし（pre-existing dead_code warningのみ）
- `cargo fmt --all -- --check` — フォーマット準拠
