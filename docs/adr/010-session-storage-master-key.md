# ADR-010: masterKey の sessionStorage 保持とページリロード対応

**ステータス:** Accepted
**決定日:** 2026-03-11
**関連:** `docs/progress/07-web-design.md §5`, `docs/progress/08-web-implementation.md`, ADR-001, ADR-003

---

## Context

Argon2id は計算コストが高い（設計パラメーター: t=2, m=19456, p=1、実行時間 ~1–2 秒）。
ページ遷移やリロードのたびに再実行するのは UX 上問題。masterKey をどこかに保持する必要がある。

検討した選択肢:

**案A: sessionStorage に raw bytes（Base64）で保持**
タブを閉じると自動消去。同一タブのページリロードでは維持される。

**案B: IndexedDB の non-extractable CryptoKey**
`crypto.subtle.importKey(..., extractable: false)` で保存。JS から raw bytes を取り出せないため、XSS でも masterKey の raw bytes が漏洩しない。タブをまたいで保持可能。

**案C: メモリのみ（React state）**
最もセキュアだが、ページリロードで消える。`router.navigate` でも state が消えることがある。

**案D: localStorage（禁止）**
ブラウザを閉じても残るため、XSS 攻撃でシークレットが永続的に漏洩するリスクがある。

---

## Decision

**案A を採用: masterKey を Base64 エンコードして `sessionStorage` に保持する。**

- キー名: `solidrop_mk`
- 保存タイミング: ログイン成功後（`/api/v1/files` への認証確認が成功した後）
- 消去タイミング: ログアウト操作、またはタブを閉じた時（ブラウザの sessionStorage 仕様による自動消去）
- `localStorage` への保存は実装・設計上明示的に禁止する

`apiToken`（masterKey から HKDF 導出）はセッション中の React state のみに保持し、sessionStorage や localStorage には保存しない。
ページリロード時は masterKey が sessionStorage から復元された後、非同期で `deriveApiToken` を再実行して apiToken を再導出する。

---

## Rationale

**なぜ sessionStorage か:**

| 観点 | sessionStorage | IndexedDB (non-extractable) | メモリのみ |
|---|---|---|---|
| XSS への耐性 | Base64 文字列として取得可能 | raw bytes 取得不可 | 取得可能（state として） |
| リロード耐性 | あり | あり | なし |
| 実装複雑度 | 低（getItem/setItem） | 高（非同期 API、エラー処理） | 低 |
| タブ間共有 | なし（タブ独立） | あり（同一オリジン内） |なし |
| 個人ツールとしての許容性 | 十分 | オーバースペック | 不便 |

本システムは**シングルユーザーの個人ツール**であり、XSS の脅威モデルは主に「自分以外がアクセスできないデバイスで動作させる」を前提とする。IndexedDB の non-extractable key が提供する追加セキュリティは、個人用途での利便性低下に見合わない。

**apiToken を sessionStorage に保存しない理由:**

masterKey が sessionStorage にあれば、apiToken はいつでも HKDF で再導出できる（数ミリ秒）。
「保存しないで済むものは保存しない」原則に従い、apiToken はリロード後に都度再導出する。

**`isAuthenticated` の判定について:**

`apiToken` は非同期で導出されるため、`isAuthenticated: !!masterKey && !!apiToken` とするとリロード直後に apiToken が null の間 `isAuthenticated: false` となり、ルーターが `/login` へ一瞬リダイレクトするフラッシュが発生する。

これを避けるため `isAuthenticated: !!masterKey` とし、apiToken の準備状況は API フック側で `enabled: !!apiToken` により制御する。
masterKey がある（= ログイン済み）がまだ apiToken が準備できていない状態では、ファイル一覧がロード中表示になるが `/login` へのリダイレクトは発生しない。

---

## Consequences

**正の結果:**
- リロード後もログイン状態が維持される（ユーザーはパスワードを再入力しなくてよい）
- タブを閉じると自動的にセッションが終了する（セキュリティ境界として機能）
- 実装がシンプル（`sessionStorage.getItem` / `setItem` / `removeItem` のみ）

**負の結果:**
- XSS が発生した場合、masterKey の Base64 文字列が取得される可能性がある（IndexedDB non-extractable と比較した場合の劣後点）
  → 個人用途・自己管理デバイスでの運用を前提に許容。CSP（Content Security Policy）の設定で XSS リスク自体を低減することが推奨される（Phase 2 以降の課題）
- 複数タブを開いた場合、それぞれのタブで独立した sessionStorage が使われる（タブ間で state が共有されない）
  → 個人ツールとして問題なし
- sessionStorage に masterKey が残っている間は、同一タブ内の任意の JS がアクセスできる
  → ブラウザ拡張機能のリスクは存在するが、個人ツールとして許容
