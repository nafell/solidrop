# ADR-003: HKDF派生APIトークンによるE2EE互換認証

**ステータス:** Accepted
**決定日:** 2026-03
**関連:** `docs/design/architecture.md §API Authentication`, `crates/crypto/SPEC.md`
**解決したTBD:** TBD-3（APIキーの生成・管理方法）

---

## Context

APIサーバーへの認証方式が未決定だった（TBD-3）。
ユーザーはE2EE保証を明示的に要求しており、「APIサーバーが暗号文を復号できる能力を持たないこと」を確認したいと述べた。

ADR-001でマスター鍵からHKDFでファイル鍵を導出する設計が確定している。
APIキーをどのように生成・管理するかには以下の選択肢があった:

**案A: 独立したランダムAPIキー**
マスター鍵とは無関係の乱数を生成し、サーバーの環境変数に設定する。

**案B: HKDF派生APIトークン**
マスター鍵からHKDFでAPIトークンを導出し、サーバーの環境変数に設定する。

---

## Decision

**APIトークンはマスター鍵からHKDF-SHA256で導出する。ドメイン分離のためinfoストリングを用いる。**

```
master_key
  ├─ HKDF(info="solidrop-api-auth")        → api_token  (32バイト)  ← サーバーが保持
  └─ HKDF(info="solidrop-file-encryption", salt=per_file_salt) → file_key (32バイト) ← クライアントのみ
```

セットアップ手順:
1. ユーザーがパスフレーズを入力
2. CLI: Argon2id → master_key を導出
3. CLI: HKDF(master_key, info="solidrop-api-auth") → api_token を導出
4. ユーザーがサーバーの環境変数 `API_KEY=<api_token_hex>` に設定

ランタイム（クライアント）:
1. ユーザーがパスフレーズを入力
2. Argon2id → master_key → HKDF("solidrop-api-auth") → api_token
3. `Authorization: Bearer <api_token>` でAPIリクエスト

---

## Rationale

**HKDF派生トークンはE2EEを侵害しないか？**

これが本ADRの核心的な問いである。

HKDF（HMAC-based Key Derivation Function）の重要な性質:
- **一方向性:** `api_token = HKDF(master_key, info="solidrop-api-auth")` から `master_key` を逆算することは計算困難
- **ドメイン分離:** `info` 文字列が異なれば、同じ `master_key` から導出されても出力は独立した擬似乱数として振る舞う

したがって、サーバーが `api_token` を知っていても:
- `master_key` を逆算できない
- `file_key = HKDF(master_key, info="solidrop-file-encryption", ...)` を導出できない

| 主体 | 知っているもの | ファイル復号可否 |
|---|---|---|
| クライアント | master_key, api_token, file_key | ○ |
| APIサーバー | api_token のみ | ✗ |
| S3 | 暗号文のみ | ✗ |
| 攻撃者（サーバー侵害） | api_token のみ | ✗（HKDF逆算不可） |

**なぜ独立したランダムAPIキー（案A）ではなく派生トークン（案B）を選んだか:**

| 観点 | 独立ランダムキー | HKDF派生トークン |
|---|---|---|
| ユーザーが管理するシークレット数 | パスフレーズ + APIキーの2つ | パスフレーズの1つ |
| セットアップ体験 | 別途APIキーを生成・保存する必要がある | パスフレーズから自動導出 |
| E2EE保証の強さ | 同等（どちらもサーバーは復号不可） | 同等 |
| キーローテーション | 独立して変更可能 | マスター鍵変更と連動 |

単一ユーザーの個人ツールとして、ユーザーが管理するシークレットを1つに絞る利便性を優先した。
パスフレーズ1つですべてのセキュリティが決まる、というシンプルなモデルが個人用途に適合する。

**サーバーに保存するのは api_token の平文か、ハッシュか？**

Bearer Token認証の照合は以下のような等値比較で行われる:
```
timing_safe_eq(received_token, stored_api_token)
```
ハッシュ化した場合は `timing_safe_eq(SHA256(received), SHA256(stored))` となるが、
これは `api_token` をサーバーに送ることに変わりなく、ハッシュ化しても本質的なセキュリティ向上はない。
シンプルに `api_token` の平文を環境変数で保持する。

---

## Consequences

**正の結果:**
- E2EEが保証される: サーバーが api_token を知っていても file_key を導出できない（HKDFの一方向性）
- ユーザーが覚えるシークレットがパスフレーズ1つに統一される
- TBD-3 が解決される: APIキー生成・管理の手順が明確になる
- ドメイン分離により、API認証用途とファイル暗号化用途の鍵が混在しない

**負の結果:**
- マスターパスワードを変更した場合、APIトークンも変わるためサーバー側の `API_KEY` も更新が必要
  → 個人ツールとして許容範囲。パスワード変更は稀なイベント
- 将来マルチデバイス・マルチユーザーへ発展する場合、この設計は適合しない
  → 本プロジェクトはシングルユーザー設計であり、スコープ内の制約として受け入れる
