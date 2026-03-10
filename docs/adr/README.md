# Architecture Decision Records (ADR)

本プロジェクトにおける重要なアーキテクチャ判断を記録する。
各ADRは「なぜその判断をしたか」の理由に焦点を当てる。

「何を決めたか（仕様）」は `docs/design/architecture.md` を参照。

## フォーマット

```
## Context   — その判断が必要になった背景・制約
## Decision  — 何を選択したか
## Rationale — なぜその選択肢を選んだか（代替案との比較含む）
## Consequences — その判断がもたらす結果（正・負）
```

## 判断ステータス

- **Accepted** — 採用済み。実装上の根拠となる
- **Superseded** — より新しいADRに置き換えられた
- **Proposed** — 議論中

## 一覧

| ID | タイトル | ステータス | 決定日 |
|---|---|---|---|
| [ADR-001](001-client-side-e2e-encryption.md) | クライアントサイドE2E暗号化の採用 | Accepted | 2026-02 |
| [ADR-002](002-api-control-plane-presigned-urls.md) | APIサーバーをコントロールプレーンに限定（プレサインドURL方式） | Accepted | 2026-02 |
| [ADR-003](003-hkdf-derived-api-token.md) | HKDF派生APIトークンによるE2EE互換認証 | Accepted | 2026-03 |
| [ADR-004](004-s3-as-source-of-truth.md) | サーバーサイドDB不使用・S3をシングルソースオブトゥルースとする | Superseded (by ADR-006) | 2026-02 |
| [ADR-006](006-valkey-metadata-index.md) | API層にValkey永続インデックスを導入し、S3メタデータ参照を最適化する | Accepted | 2026-03 |
| [ADR-005](005-lru-cache-approval-eviction.md) | iPadローカルストレージをLRUキャッシュとして扱い、承認制で退避する | Accepted | 2026-02 |
| [ADR-007](007-web-crypto-noble-hashes-hybrid.md) | ブラウザ暗号化実装に Web Crypto API と @noble/hashes のハイブリッドを採用 | Accepted | 2026-03 |
| [ADR-008](008-vite-master-salt-env.md) | Argon2id マスター鍵導出ソルトをビルド時環境変数で注入する | Accepted | 2026-03 |
| [ADR-009](009-caddy-same-origin-proxy.md) | Caddy 同一オリジンプロキシによる CORS 不要設計 | Accepted | 2026-03 |
| [ADR-010](010-session-storage-master-key.md) | masterKey の sessionStorage 保持とページリロード対応 | Accepted | 2026-03 |
