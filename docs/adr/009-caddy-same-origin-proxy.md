# ADR-009: Caddy 同一オリジンプロキシによる CORS 不要設計

**ステータス:** Accepted
**決定日:** 2026-03-11
**関連:** `docs/progress/07-web-design.md §13`, `infra/vps/Caddyfile.web-staging.snippet`, ADR-002

---

## Context

Web フロントエンドと API サーバーをどのドメインに配置するかで CORS 設計が変わる。

**構成 A: 別サブドメイン**
- フロントエンド: `staging.web.solidrop.nafell.dev`
- API: `staging.api.solidrop.nafell.dev`
- → 別オリジン → CORS ヘッダーが必要

**構成 B: 同一ドメイン（フロントエンドが /api/* を API へプロキシ）**
- フロントエンド + API: `staging.web.solidrop.nafell.dev`（Caddy がルーティング）
- → 同一オリジン → CORS 不要

CLI は `staging.api.solidrop.nafell.dev` を直接呼ぶため、API ドメインは残す必要がある。

---

## Decision

**Web フロントエンド用ドメイン（`staging.web.*` / `web.*`）では、Caddy が `/api/*` と `/health` を API コンテナに透過するリバースプロキシを設定する。**

```caddyfile
staging.web.solidrop.nafell.dev {
    handle /api/* {
        reverse_proxy http://localhost:3001   # API staging
    }
    handle /health {
        reverse_proxy http://localhost:3001
    }
    handle {
        reverse_proxy http://localhost:3011   # nginx (静的ファイル)
    }
}
```

- フロントエンドの API 呼び出しは相対パス `/api/v1/...` を使う
- ブラウザから見ると同一オリジン → `Authorization` ヘッダーを含む CORS プリフライトが不要
- API サーバーの `CorsLayer` 設定変更不要

---

## Rationale

**CORS を回避する意味:**

CORS はクロスオリジンリクエストのセキュリティ機構。設定が複雑で、ミスが XSS・CSRF リスクにつながる可能性がある。
同一オリジンで配信できる場合は、CORS を設定しないことが最もシンプルかつ安全。

**Caddy プロキシによる同一オリジン設計のトレードオフ:**

| 観点 | 同一オリジン（Caddy プロキシ） | 別オリジン（CORS 設定） |
|---|---|---|
| 設定の複雑さ | Caddy 設定のみ | API サーバーの CORS 許可リストも必要 |
| セキュリティ | same-origin で Cookie・ヘッダー制約が自動適用 | CORS 設定ミスのリスク |
| CLI との共存 | `staging.api.*` は CLI 専用として残す | 変更なし |
| ローカル開発 | Vite の dev proxy (`vite.config.ts`) で模倣 | 同様に proxy 設定が必要 |

**ADR-002 との関係:**

ADR-002 はファイルデータが API サーバーを経由しないことを保証する（プレサインド URL 方式）。
本 ADR の Caddy プロキシは API の「制御メッセージ」（presign リクエスト・一覧取得等）のみを透過し、S3 へのファイル転送は引き続きクライアント→S3 直接。
プレサインド URL（S3 ドメイン）へのリクエストはクロスオリジンだが、presigned URL は認証済みの一時 URL であり CORS プリフライトが発生しない（`fetch` で直接 PUT/GET する）。

---

## Consequences

**正の結果:**
- API サーバーコードに変更なし（`CorsLayer` 設定を Web 向けに変更する必要がない）
- フロントエンドコードは `/api/v1/...` のような相対 URL を使えるため、環境依存が少ない
- Caddy が TLS 終端も兼ねるためフロントエンドとの通信は常に HTTPS

**負の結果:**
- Caddy の設定変更がフロントエンドの動作に影響する（Caddy が SPOF になる面がある）
- ローカル開発では `vite.config.ts` の `server.proxy` で同等設定を再現する必要がある
  ```typescript
  server: { proxy: { '/api': 'http://localhost:3000', '/health': 'http://localhost:3000' } }
  ```
- S3 presigned URL への PUT/GET はクロスオリジンのまま（S3 バケットの CORS 設定が必要な場合は別途対応）
