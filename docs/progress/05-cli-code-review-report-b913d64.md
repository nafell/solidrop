# 05 — CLI Code Review Report (`b913d6483dc03f4e8ca2b915a6a301471d230a63`)

対象コミット:
- `b913d6483dc03f4e8ca2b915a6a301471d230a63`

対象範囲:
- `crates/cli`（API client / commands / config / master key / integration tests）

## Executive Summary

CLIの主要コマンド実装は一通り揃っている一方で、**URL構築の安全性不足**、**sync保存先の衝突リスク**、**設定と実装の不整合**、**テスト戦略の実態乖離**が残っている。

---

## Findings

### 1) パス/クエリ未エンコードのURL組み立て
- Severity: High
- Category: Security / Runtime bug

#### 問題
- `delete_file` が `format!("{}/files/{}", ...)` で生の `path` をURLへ埋め込んでいる。
- 統合テスト側でも `?prefix={p}` や `/files/{path}` を手組みしており、予約文字・空白・`#`・`?` などで意図しない解釈になる。

#### 影響
- 一部キーで404/400化、誤ルーティング、削除対象取り違えのリスク。

#### 推奨
- `reqwest::Url` で path segments / query pairs を設定し、手組みURLを廃止する。

---

### 2) `sync` が basename で保存しリモート衝突を引き起こす
- Severity: Medium
- Category: Architectural consistency / Data integrity

#### 問題
- `transfer/a/report.enc` と `transfer/b/report.enc` がどちらもローカル `report` に集約される。
- 既存判定も basename ベースのため、片方が誤って `skipped` される可能性がある。

#### 影響
- 上書き・取りこぼし・冪等性の崩れ。

#### 推奨
- `download_dir` 配下にリモート相対パス構造を維持して保存する（例: `transfer/...` をサブディレクトリとして再現）。

---

### 3) `upload_dir` 設定が未使用（仕様と実装の矛盾）
- Severity: Medium
- Category: Implementation contradiction

#### 問題
- `StorageConfig` に `upload_dir` があるが、`upload::run` は `_config` で受け取り未使用。

#### 影響
- 設計意図が不透明で、ユーザー設定が効かない。

#### 推奨
- 相対パスアップロード時に `upload_dir` を適用する、または設定項目を削除して仕様を明確化する。

---

### 4) 「CLI統合テスト」がCLIを通していない
- Severity: Medium
- Category: Test completeness

#### 問題
- テストは `reqwest` でAPIを直接叩いており、CLIバイナリの引数解析・設定読み込み・環境変数取り扱い・コマンド配線を検証していない。

#### 影響
- 実運用経路（`solidrop` コマンド実行）での回 regressions を見逃す。

#### 推奨
- `assert_cmd` 等でCLIバイナリを起動するE2Eテストを追加し、既存のAPI直叩きテストはAPI契約テストとして位置づける。

---

### 5) master key テストの環境変数競合（並列実行時）
- Severity: Low
- Category: Test reliability

#### 問題
- 同一env (`SOLIDROP_MASTER_KEY`) を複数テストで set/remove しており、並列実行で競合し得る。

#### 影響
- CIでのフレーク化。

#### 推奨
- `serial_test` で直列化、または env 依存部分を pure function に分離して単体テストする。

---

## 補足

このレビューはコミット単位の静的レビューであり、外部依存（crates.io）到達性制限のためローカルで `cargo test -p solidrop-cli` は完走できなかった。
