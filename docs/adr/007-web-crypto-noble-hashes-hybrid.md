# ADR-007: ブラウザ暗号化実装に Web Crypto API と @noble/hashes のハイブリッドを採用

**ステータス:** Accepted
**決定日:** 2026-03-11
**関連:** `docs/progress/07-web-design.md`, `docs/progress/08-web-implementation.md`, ADR-001
**解決したTBD:** ブラウザでの暗号化実装方式

---

## Context

ADR-001 でクライアントサイド E2EE を採用し、Rust の `solidrop-crypto` クレートで暗号化ロジックを実装した。
Web フロントエンドを追加する際、同じ暗号仕様（Argon2id → AES-256-GCM、HKDF-SHA256）をブラウザで再現する必要があった。

検討した実装方式は以下の 3 つ:

**方式 A: Web Crypto API のみ**
`SubtleCrypto` はブラウザネイティブの暗号化 API。AES-256-GCM、HKDF-SHA256、SHA-256 に対応するが、**Argon2id は非対応**。

**方式 B: wasm-pack で solidrop-crypto を WASM 化**
`solidrop-crypto` を `wasm-pack` でコンパイルしてブラウザから呼び出す。ロジックの完全共有が可能。

**方式 C: JS ライブラリ（@noble/hashes 等）のみ**
`@noble/hashes` は Argon2id を含む全暗号プリミティブを JS で実装。Web Crypto API を一切使わない。

**方式 D（採用）: Web Crypto API + @noble/hashes ハイブリッド**
Argon2id のみ `@noble/hashes` を使い、その他（AES-GCM、HKDF、SHA-256）は Web Crypto API を使う。

---

## Decision

**AES-256-GCM・HKDF-SHA256・SHA-256 は Web Crypto API (SubtleCrypto) を使う。Argon2id のみ `@noble/hashes` を使う。**

| 操作 | 実装 |
|---|---|
| Argon2id（masterKey 導出） | `@noble/hashes/argon2` |
| HKDF-SHA256（fileKey / apiToken 導出） | `SubtleCrypto.deriveBits` |
| AES-256-GCM（暗号化・復号） | `SubtleCrypto.encrypt` / `decrypt` |
| SHA-256（content_hash） | `SubtleCrypto.digest` |

wasm-pack による `solidrop-crypto` の WASM 化は将来の選択肢として保留する。

---

## Rationale

**なぜ Web Crypto API を優先するか:**

- **ゼロ追加依存:** AES-GCM・HKDF・SHA-256 はブラウザネイティブで実装されており、追加パッケージが不要。
- **ハードウェアアクセラレーション:** Web Crypto API は OS・CPU の暗号化命令（AES-NI 等）を利用できる。JS 実装より数倍から数十倍高速。
- **FIPS 140-2 準拠:** ブラウザ組み込み実装は標準準拠が保証されている。
- **セキュリティ監査コスト:** 実装を持たないため脆弱性調査の対象が減る。

**なぜ Argon2id だけ @noble/hashes か:**

Web Crypto API は Argon2id を仕様上サポートしていない（PBKDF2 と Scrypt のみ）。
`@noble/hashes` は Paul Miller 氏によるセキュリティ重視の実装で、`argon2id` をピュア JS で提供する。
監査実績があり、個人プロジェクト規模での採用リスクは低い。

**なぜ wasm-pack（方式 B）を今選ばないか:**

| 観点 | wasm-pack | ハイブリッド |
|---|---|---|
| ロジック共有 | Rust コードをそのまま再利用 | JS で再実装（テストで互換性確認） |
| ビルドチェーン | cargo + wasm-pack + wasm-opt が必要 | npm のみ |
| バンドルサイズ | WASM バイナリ分追加（~100KB 程度） | @noble/hashes のみ（~40KB gzip 込み） |
| 初期コスト | Dockerfile・CI 改修が必要 | 不要 |

Phase 1 の実装スコープでは wasm-pack 導入コストに見合わない。将来、Flutter・Web・CLI でロジックを完全共有する必要が生じた時点で再評価する（DEF-x）。

**なぜ JS ライブラリのみ（方式 C）を選ばないか:**

Web Crypto API が利用可能な環境ではネイティブ実装が優先される。特に AES-GCM は 55MB ファイルの暗号化・復号で実行時間に影響するため、ハードウェアアクセラレーションの恩恵は大きい。

---

## Consequences

**正の結果:**
- ビルド依存が `@noble/hashes` 1 パッケージの追加のみ（wasm-pack ツールチェーン不要）
- AES-GCM・HKDF はブラウザネイティブ実装でパフォーマンス最大化
- Argon2id の実装は CLI（Rust）と JS で分離されるが、同一パラメーター（t=2, m=19456, p=1）と同一入力で同一出力が得られることをテストで確認可能

**負の結果:**
- Argon2id の実装が Rust と JS で二重管理になる。パラメーター変更時は両方の更新が必要
- `@noble/hashes` の脆弱性が発生した場合は web のみが影響を受ける（CLI は独立）
- Web Crypto API と @noble/hashes の出力互換性はテストによる保証が必要（単体テスト未作成、TBD）
