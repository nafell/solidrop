# 04 — Code Review Report (Architecture / Security / Runtime / Tests)

対象:
- `docs/progress/02-tbd-decisions-report.md`
- `docs/progress/03-api-endpoints-report.md`
- `crates/api-server` 実装

## Executive Summary

現状のAPIサーバーは基本機能は動作するが、**運用時に障害を誤判定する実装**、**オブジェクトキー互換性の欠陥**、**移動処理の一貫性不備**、**LRU判定の時刻処理の脆弱性**、**テスト未カバー領域**が残っている。

---

## Findings

## 1) `DELETE /api/v1/files/*path` が内部障害を 404 に誤変換する
- Severity: High
- Category: Error handling / observability

### 問題
`head_object` の失敗をすべて `NotFound` にマッピングしており、認可エラー・タイムアウト・接続障害なども 404 として返る。

### 根拠
- `delete_file` の存在確認で `map_err(|_| AppError::NotFound(...))` を使用している。  
  (`crates/api-server/src/routes/delete.rs`)

### 影響
- 実障害時に「ファイルがない」と誤認し、調査が遅れる。
- クライアント側のリトライ/復旧戦略を誤らせる。

### 推奨
- S3エラー型を判別して `NoSuchKey` のみ 404、それ以外は 5xx を返す。
- 監視向けに内部エラーを区別してログ出力する。

---

## 2) `POST /api/v1/files/move` の `copy_source` がURLエンコード非対応
- Severity: High
- Category: Runtime bug / compatibility

### 問題
`copy_source(format!("{bucket}/{}", body.from))` をそのまま送っている。S3の `x-amz-copy-source` はURLエンコード要件があり、スペースや一部記号を含むキーで失敗する可能性がある。

### 根拠
- 生の文字列連結で `copy_source` を構築している。  
  (`crates/api-server/src/routes/file_move.rs`)

### 影響
- 一部ファイル名（空白、Unicode、記号）で移動APIが不安定化。
- クライアントから見ると「特定ファイルだけ移動できない」断続障害になる。

### 推奨
- AWS SDK推奨形式で `copy_source` を正規化/エンコードして設定する。
- キー文字種ケースの統合テストを追加する。

---

## 3) `POST /api/v1/files/move` はコピー後削除失敗時に整合性が崩れる
- Severity: Medium
- Category: Architectural consistency / data consistency

### 問題
処理が `copy -> delete` の2段階で、削除失敗時は500を返すが、コピーは完了済みのため「移動失敗レスポンスなのに実体は複製済み」という半端状態になる。

### 根拠
- `copy_object` 成功後に `delete_object` を実行。失敗時は `Internal` で終了。  
  (`crates/api-server/src/routes/file_move.rs`)

### 影響
- クライアント再試行で重複/想定外上書きが発生しうる。
- 「move」を原子的操作とみなす呼び出し側で実装矛盾が起きる。

### 推奨
- API仕様として「best-effort move（コピー後削除）」を明示するか、
- 条件付きコピー/冪等キー/補償処理を導入し、再試行安全性を上げる。

---

## 4) キャッシュLRU判定が文字列ソート依存で時刻解釈が脆弱
- Severity: Medium
- Category: Runtime logic bug

### 問題
`last_used` を文字列比較でソートしている。ISO8601固定フォーマット前提が崩れると時系列順が保証されない。

### 根拠
- `sorted.sort_by(|a, b| a.last_used.cmp(&b.last_used));`  
  (`crates/api-server/src/routes/cache.rs`)

### 影響
- LRU候補選定が誤り、削除優先順位が逆転しうる。

### 推奨
- 受信時に日時型へパース（不正値は400）。
- サーバー側で正規化した時刻比較に統一する。

---

## 5) テスト網羅が成功系寄りで失敗系・境界系が不足
- Severity: Medium
- Category: Test completeness

### 問題
S3が必要な6テストが `#[ignore]` で通常実行から外れている。さらに、主要な異常系が未検証。

### 根拠
- S3統合テストに `#[ignore]` が付与されている。  
  (`crates/api-server/tests/api_test.rs`)

### 未カバー例
- `move` の `copy_source` 文字種境界（space/utf-8/reserved chars）
- `delete` での `403/timeout` 等エラー分類
- `presign` / `move` の入力バリデーション境界（空白のみ、長大キーなど）
- `files` の `limit` / `next_token` / `head_object` 失敗時挙動

### 推奨
- CIでMinIO統合テストを定期実行（nightlyでも可）。
- 異常系テストを追加し、ステータスコード契約を固定化する。

---

## Progress reportとの整合コメント

`03-api-endpoints-report.md` では「API Serverが完全に機能する状態」と記載があるが、上記のとおりエラー分類と一貫性の観点で未解決事項があるため、表現は「主要機能を実装済み。運用向けの堅牢化項目が残る」が実態に近い。
