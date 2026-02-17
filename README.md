# SoliDrop — iPad お絵かきデータ基盤 要件定義書・設計書

> **ドキュメントバージョン:** 0.1.0（Phase 1 MVP 着手前）
> **最終更新:** 2026-02-11
> **ステータス:** レビュー済み・実装着手可

---

## 目次

1. [プロジェクト概要](#1-プロジェクト概要)
2. [議論経緯と設計判断ログ](#2-議論経緯と設計判断ログ)
3. [機能要件](#3-機能要件)
4. [非機能要件](#4-非機能要件)
5. [システムアーキテクチャ](#5-システムアーキテクチャ)
6. [フロントエンド設計](#6-フロントエンド設計)
7. [バックエンドAPI設計](#7-バックエンドapi設計)
8. [インフラ設計](#8-インフラ設計)
9. [暗号化設計](#9-暗号化設計)
10. [通信設計](#10-通信設計)
11. [データモデル](#11-データモデル)
12. [キャッシュ戦略（ストレージオフロード）](#12-キャッシュ戦略ストレージオフロード)
13. [復元フロー](#13-復元フロー)
14. [コスト試算](#14-コスト試算)
15. [技術スタック](#15-技術スタック)
16. [プロジェクト構成](#16-プロジェクト構成)
17. [開発フェーズ](#17-開発フェーズ)
18. [未決定事項・保留事項](#18-未決定事項保留事項)

---

## 1. プロジェクト概要

### 1.1 目的

iPadでのお絵かき作業の効率化のため、データ基盤（PCとの送受信、iPad本体ストレージ128GB制限への対応）で支援する。

### 1.2 スコープ

- ユーザーは自分ひとり。共同作業・外部公開は考慮しない。
- 個人のお絵かきUX向上が最優先目的。
- 副次的に、技術スタック（特にRust、ファイル同期、システム設計、AWS）の学習を行う。

### 1.3 使用ツール（システムが支援する対象）

| カテゴリ | ツール | 用途 | データ形式 |
|---|---|---|---|
| iPadクリエイティブ（最優先） | Clip Studio | 本腰のお絵かき | .clip（中央値30MB、最大55MB） |
| iPadクリエイティブ | Procreate | 落書き・自由な描画 | .procreate |
| iPadクリエイティブ | FreeForm | アイデア整理 | - |
| iPad補助 | Files | ファイル管理 | - |
| iPad補助 | スクリーンショット | 作業途中の風景撮影、SNS投稿用 | .png |
| iPad補助 | Grok Imagine / Gemini Nano等 | アイデア出し | - |
| PC | Affinity (Windows/Mac) | 編集、印刷/入稿データ作成 | .tiff等 |
| PC | Blender | 資料用3Dモデル | - |
| Android/PC | — | 資料閲覧、アップロード、SNS投稿 | 各種画像 |

---

## 2. 議論経緯と設計判断ログ

以下は設計に至るまでの議論過程と、各判断の根拠を記録したものである。後続の開発者（LLMを含む）が「なぜこうなったか」を理解できるようにする。

### 2.1 初期方針（Gemini回答）

初期の議論相手（Gemini）が提案した主要方針:

1. **File Provider Extension を本命としたiOS同期** — クリスタからOS標準Filesアプリ経由で直接読み書き可能にする
2. **サーバーレスアーキテクチャ** — Lambda/Cloud Run + S3/GCS + DynamoDB/Firestore
3. **LAN内P2P通信（AirDrop的機能）** — mDNS + WebRTC/ローカルHTTP
4. **段階的開発** — Phase 1(一方通行バックアップ) → Phase 2(PC連携) → Phase 3(UX向上)

コスト試算は月額$5以下と見積もった。

### 2.2 アーキテクトレビューによる修正

レビュー（Claude）で以下の問題を指摘し、方針を修正した:

#### 判断1: File Provider Extensionの優先度を下げる

- **理由:** iOS開発の中でもニッチで難易度が高い。ドキュメント不十分、デバッグ困難。Extension自体のメモリ制限（数十MB）があり、大容量.clipファイルの扱いにchunked upload等の工夫が必須。個人開発・学習目的で最初から着手するリスクが高い。
- **決定:** Phase 2以降に延期。MVPは手動ファイル選択で開始。

#### 判断2: リアルタイム同期は不要 → キャッシュ戦略の導入

- **理由:** ユーザー（Nafell）からの明確な要求。「保存されたら即アップロード」ではなく、「本体ストレージの上限管理 + 溢れた分のクラウド退避」が本質。
- **決定:** コンピュータアーキテクチャのLRUキャッシュ戦略をファイル管理に適用。日次バックアップ + 使用頻度ベースの退避。

#### 判断3: コスト試算の修正

- **理由:** Geminiの$5/月試算は100GBストレージ前提で楽観的。実測ベースで再計算した結果、初年度は$0.4/月程度。
- **決定:** 実データに基づく試算を採用（詳細は[14. コスト試算](#14-コスト試算)）。

#### 判断4: P2P機能の優先度を下げる（YAGNI）

- **理由:** LAN内P2P実装はiPad側・PC側両方に開発工数がかかり、LAN外のフォールバックも必要。クラウド経由の署名付きURL方式で体感速度は十分実用的。
- **決定:** Phase 2以降。クラウド経由で遅いと感じてから検討する。

#### 判断5: MVPではDB不要

- **理由:** 個人利用でファイルパス→メタデータ程度ならS3のListObjects API + オブジェクトメタデータタグで十分。DynamoDB/Firestoreを別途立てる複雑さが学習コスト・運用コストに見合わない。
- **決定:** クラウド側DBなし。クライアント側のキャッシュ状態管理はローカルSQLite。

#### 判断6: 復元フローの重要性

- **指摘:** バックアップシステムで最も価値があるのは「保存」ではなく「復元」。何クリックで戻せるかがシステムの価値を決める。
- **決定:** 復元フローを設計に含める。MVPではバージョニングUIは後回しだが、S3バージョニングは有効にしておく。

#### 判断7: クライアントサイド暗号化の採用

- **ユーザー要求:** 「自分しか復号できないようにしたい」
- **トレードオフ:** サーバー側でサムネイル生成やプレビューができなくなる。
- **決定:** ファイル本体はクライアントサイドAES-256-GCM暗号化。メタデータ（ファイル名、日時、サイズ）は平文で管理（ユーザー了承済み）。

#### 判断8: インフラ構成 — XServer VPS + AWS S3

- **経緯:** 当初はFargate想定。しかし既存契約のXServer VPS（6GB RAM）があるため、APIサーバーはVPS上のDockerで動かし、ストレージのみAWS S3を使用する構成を検討。
- **評価:** 署名付きURL方式ではAPIサーバーのS3間データ転送は極めて少量（URLの発行のみ）なので、VPSがAWS外にあってもレイテンシ・Egress料金への影響は軽微。追加コスト0で開始できる利点が大きい。
- **注意点:** VPS上にIAMアクセスキーを配置する必要があり、VPSのセキュリティがAWSアカウントに直結する。IAM権限の最小化とアクセスキーのローテーション運用が必須。 - **決定:** Phase 1はXServer VPS。将来的にFargate/Lambdaへの段階的移行を想定。Dockerコンテナで実装し、移行容易性を確保する。
#### 判断9: 技術スタック選定

- **サーバーサイド:** Rust（学習目的、第一優先）。次点Go。
- **クライアント（iPad/Android）:** Flutter。ただしPhase 2以降のFile Provider ExtensionはSwift必須となる点を認識。
- **PC CLIツール:** Rust。
- **Webフロントエンド:** 必要時はRuby（ただし現時点ではスコープ外）。
- **インフラ:** AWS。ベンダーロックインを最小限にする方針。

### 2.3 iOSサンドボックス制約に関する重要な前提

クリスタの.clipファイルは `On My iPad/Clip Studio` 配下に保存される。iOSのサンドボックス制約上、他のアプリからこのディレクトリを直接読み取ることは**不可能**。

MVPでは以下の運用で対処する:

- ユーザーがFiles.app経由で自作アプリのアクセス可能領域にファイルをコピー/移動
- または、自作アプリ内のファイルピッカーで直接選択してアップロード

File Provider Extensionによるシームレスな統合はPhase 2以降。

---

## 3. 機能要件

### 3.1 機能A: ストレージオフローダー（キャッシュ戦略）

iPad本体の128GBストレージを有効活用するため、使用頻度の低いファイルをクラウドに退避し、必要時に復元する。

| ID | 要件 | 優先度 | Phase |
|---|---|---|---|
| A-1 | 手動でファイルを選択し、暗号化してS3にアップロードできる | 必須 | 1 |
| A-2 | クラウド上のファイル一覧を閲覧できる | 必須 | 1 |
| A-3 | クラウドからファイルをダウンロードし復号して保存できる | 必須 | 1 |
| A-4 | 日次の自動バックアップ（BGTaskScheduler） | 必須 | 1 |
| A-5 | ローカルストレージ使用量の上限設定と退避候補の提示 | 必須 | 1 |
| A-6 | 退避候補の承認後、自動でクラウド退避＋ローカル削除 | 必須 | 1 |
| A-7 | S3バージョニングによる世代管理UIでの復元 | 望ましい | 2 |
| A-8 | File Provider Extensionによるクリスタ直接統合 | 望ましい | 3 |

### 3.2 機能B: ファイル転送パイプライン

デバイス間のファイル送受信。資料画像の送受信、完成tiffのPC連携。

| ID | 要件 | 優先度 | Phase |
|---|---|---|---|
| B-1 | iPadからファイルを選択し、暗号化してS3にアップロード | 必須 | 1 |
| B-2 | PC CLIツールで新着ファイルのダウンロード＋復号 | 必須 | 1 |
| B-3 | PC CLIツールからファイルをアップロード（Affinity編集後のPC→iPad） | 必須 | 1 |
| B-4 | PC用GUIアプリ | 望ましい | 3 |
| B-5 | Android用アプリ（資料閲覧・アップロード） | 望ましい | 3 |
| B-6 | LAN内P2P直接転送（AirDrop的） | 望ましい | 3 |

---

## 4. 非機能要件

| ID | 要件 | 決定事項 | 備考 |
|---|---|---|---|
| NF-1 | 通信経路暗号化 | HTTPS/TLS 1.3 必須 | — |
| NF-2 | データ本体の暗号化 | クライアントサイド AES-256-GCM | サーバー側プレビュー不可のトレードオフ受容済み |
| NF-3 | メタデータの暗号化 | 平文（SSE-S3のみ） | ファイル名が見られても問題ないとユーザー確認済み |
| NF-4 | 認証 | APIキー（Bearer Token）、単一ユーザー | 複数ユーザー対応は不要 |
| NF-5 | 開発手法 | MVP優先、アジャイル・インクリメンタル | 開発着手可能性を最重視 |
| NF-6 | ベンダーロックイン | 最小化方針 | S3互換API抽象化、Docker化 |
| NF-7 | 可用性 | 個人用途のため厳密なSLA不要 | VPSダウン時はアップロード不可だがデータ損失なし |

---

## 5. システムアーキテクチャ

### 5.1 構成図（Phase 1 MVP）

```
┌─────────────────────────┐       ┌──────────────────────┐
│            iPad (Flutter App)            │       │             PC (Rust CLI)          │
│                                          │       │                                    │
│          ・手動ファイル選択                │       │           ・新着ポーリング         │
│          ・AES-256-GCM暗号化             │       │           ・復号＋保存             │
│          ・署名付きURLでS3 PUT            │       │           ・アップロード           │
│          ・日次自動バックアップ             │       │                                    │
│          ・キャッシュ管理                  │       │                                    │
│            (ローカルSQLite)               │       │                                    │
└───────────┬─────────────┘       └──────────┬───────────┘
                    │                                             │
                    │  HTTPS (署名付きURL)                         │  HTTPS (署名付きURL)
                    ▼                                             ▼
┌────────────────────────────────────────────────────────┐
│                      AWS S3                                                                 │
│   Bucket: {project-name}-art-storage                                                        │
│   ├── /active/      ← ローカルにも存在するファイル                                             │
│   ├── /archived/    ← クラウドのみ（退避済み）                                                │
│   └── /transfer/    ← デバイス間転送用                                                       │
│                                                                                             │
│   設定: SSE-S3暗号化有効、バージョニング有効                                                      │
│   Lifecycle: /archived/ → 90日後 Glacier Instant                                             │
└────────────────────────────────────────────────────────┘
            ▲
            │  HTTPS (署名付きURL発行リクエスト)
            │
┌────────────────────────────────────────────────────────┐
│              XServer VPS (Docker)                                                            │
│                                                                                              │
│   ┌────────────────────────────────────┐                           │
│   │   Rust API Server (axum)                                   │                           │
│   │                                                            │                           │
│   │   ・署名付きURL発行                                          │                           │
│   │   ・ファイル一覧 (S3 ListObjects)                            │                           │
│   │   ・キャッシュ状態管理API                                     │                           │
│   │   ・Bearer Token認証                                        │                           │
│   └────────────────────────────────────┘                           │
│                                                                                             │
│   IAMアクセスキー（最小権限: 特定バケットのみ）                                                   │
└────────────────────────────────────────────────────────┘
```

### 5.2 データフロー

**アップロード（iPad → S3）:**

```
1. ユーザーがアプリ内でファイルを選択
2. SHA-256ハッシュ計算（平文に対して）
3. AES-256-GCM暗号化（マスターキーから派生した鍵）
4. APIサーバーに署名付きURL発行リクエスト（path, content_hash, size_bytes）
5. APIサーバーがS3署名付きURLを生成して返却
6. クライアントが署名付きURLで暗号文をS3に直接PUT
7. ローカルSQLiteのfile_cacheテーブルを更新
```

**ダウンロード（S3 → PC/iPad）:**

```
1. APIサーバーからファイル一覧取得
2. ダウンロード対象を選択
3. APIサーバーに署名付きダウンロードURL発行リクエスト
4. 署名付きURLでS3から暗号文をGET
5. AES-256-GCM復号
6. SHA-256ハッシュ検証（整合性確認）
7. ファイルとして保存
```

---

## 6. フロントエンド設計

### 6.1 iPad / Android（Flutter）

**MVP機能:**

- ファイル選択UI（iOSのドキュメントピッカー連携）
- アップロード進捗表示
- クラウドファイル一覧（パス、サイズ、日時の表示。サムネイルなし — クライアントサイド暗号化のため）
- ダウンロード＋復号＋Files.appアクセス可能領域への保存
- ストレージ使用量表示と退避候補の通知・承認UI
- 日次自動バックアップ（BGTaskScheduler）
- マスターパスワード入力 / Keychain連携

**iOSサンドボックス制約への対応:**

- クリスタの保存領域には直接アクセス不可
- ファイルピッカー経由でユーザーが手動選択する運用
- 自アプリの管理領域内ファイルに対してのみキャッシュ管理を自動適用

### 6.2 PC（Rust CLIツール）

**MVP機能:**

```
solidrop upload <file_path>           # ファイルをアップロード
solidrop download <remote_path>       # ファイルをダウンロード
solidrop list [--prefix <prefix>]     # ファイル一覧
solidrop sync                         # 新着ファイルをダウンロード
solidrop delete <remote_path>         # ファイル削除
solidrop move <from> <to>             # ファイル移動（active ↔ archived）
```

- 設定ファイル（TOML）: APIエンドポイント、APIキー、ダウンロード先ディレクトリ、マスターパスワードのキーチェーン参照先
- OS のクレデンシャルストア（Windows: Credential Manager、Mac: Keychain）にマスターキーを保持

---

## 7. バックエンドAPI設計

### 7.1 認証

- Bearer Token（APIキー）方式
- 全エンドポイントで `Authorization: Bearer <token>` ヘッダ必須
- TLS必須

### 7.2 エンドポイント一覧（Phase 1 MVP）

#### `POST /api/v1/presign/upload`

署名付きアップロードURL発行。

```json
// Request
{
  "path": "active/2026-02/illustration-01.clip.enc",
  "content_hash": "sha256:abc123...",
  "size_bytes": 31457280
}

// Response 200
{
  "upload_url": "https://s3.ap-northeast-1.amazonaws.com/...",
  "expires_in": 3600
}
```

- `content_hash` が既存オブジェクトと一致する場合、重複アップロードをスキップ可能（dedup）。
  - ※ 暗号化後のハッシュではなく、**平文に対するハッシュ**を送信する設計。サーバー側でdedup判定に使用する。サーバーはハッシュ値のみ保持し、平文データには一切触れない。

#### `POST /api/v1/presign/download`

署名付きダウンロードURL発行。

```json
// Request
{
  "path": "archived/2025-12/old-work.clip.enc"
}

// Response 200
{
  "download_url": "https://s3.ap-northeast-1.amazonaws.com/...",
  "expires_in": 3600
}
```

#### `GET /api/v1/files`

ファイル一覧取得。

```
GET /api/v1/files?prefix=active/&limit=100&next_token=...
```

```json
// Response 200
{
  "files": [
    {
      "path": "active/2026-02/illustration-01.clip.enc",
      "size_bytes": 31457280,
      "last_modified": "2026-02-10T15:30:00Z",
      "content_hash": "sha256:abc123..."
    }
  ],
  "next_token": "..."
}
```

- `content_hash` はS3オブジェクトのメタデータタグから取得。

#### `DELETE /api/v1/files/{encoded_path}`

ファイル削除。

```
DELETE /api/v1/files/active%2F2025-06%2Fold-sketch.clip.enc
```

```json
// Response 200
{ "deleted": true }
```

#### `POST /api/v1/files/move`

ファイル移動（active ↔ archived間）。S3のCopyObject + DeleteObjectで実装。

```json
// Request
{
  "from": "active/2025-12/old-work.clip.enc",
  "to": "archived/2025-12/old-work.clip.enc"
}

// Response 200
{ "moved": true }
```

#### `POST /api/v1/cache/report`

iPadアプリからのキャッシュ状態報告。サーバー側で退避候補を計算して返却する。

```json
// Request
{
  "local_files": [
    {
      "path": "active/2026-02/illustration-01.clip.enc",
      "content_hash": "sha256:abc123...",
      "last_used": "2026-02-11T10:00:00Z"
    }
  ],
  "storage_limit_bytes": 64424509440
}

// Response 200
{
  "evict_candidates": [
    {
      "path": "active/2025-06/old-sketch.clip.enc",
      "reason": "lru",
      "last_used": "2025-06-01T00:00:00Z"
    }
  ]
}
```

### 7.3 エラーレスポンス

```json
{
  "error": {
    "code": "FILE_NOT_FOUND",
    "message": "The specified file does not exist."
  }
}
```

標準HTTPステータスコードを使用: 400, 401, 404, 409, 500。

---

## 8. インフラ設計

### 8.1 AWSリソース

| リソース | 設定 | 備考 |
|---|---|---|
| S3 Bucket | `{project-name}-art-storage`, ap-northeast-1 | SSE-S3暗号化有効、バージョニング有効 |
| S3 Lifecycle Policy | `/archived/*` → 90日後 Glacier Instant Retrieval | 長期保存コスト削減 |
| IAM User | `solidrop-api` | 最小権限: 特定バケットに対するPutObject, GetObject, ListBucket, DeleteObject, CopyObjectのみ |
| IAM Policy | インラインポリシー | バケットARN指定 |

### 8.2 S3バケット構成

```
{project-name}-art-storage/
├── active/           ← iPadローカルにも存在するファイルのバックアップ
│   └── {YYYY-MM}/   ← 月別ディレクトリ（運用規約）
├── archived/         ← iPad本体から退避済み（クラウドのみ）
│   └── {YYYY-MM}/
└── transfer/         ← デバイス間転送用（短期保持）
    └── {YYYY-MM-DD}/
```

- ファイル名は元のファイル名 + `.enc` 拡張子（例: `illustration-01.clip.enc`）。
- ディレクトリ構成は運用規約であり、システムが強制するものではない。

### 8.3 XServer VPS（Phase 1）

| 項目 | 値 |
|---|---|
| RAM | 6GB |
| デプロイ | Docker（docker-compose） |
| TLS | Let's Encrypt (certbot) または Caddy |
| セキュリティ | SSH鍵認証のみ、UFWでHTTPS(443)のみ開放 |
| IAMキー管理 | 環境変数 or Docker secrets、定期ローテーション |

### 8.4 Terraform管理対象

Phase 1で管理するAWSリソース:

- S3 Bucket（設定含む）
- IAM User + Policy
- S3 Lifecycle Policy

VPS自体はTerraform管理外（既存契約のため手動管理）。

---

## 9. 暗号化設計

### 9.1 概要

| 対象 | 方式 | 備考 |
|---|---|---|
| 通信経路 | TLS 1.3 | 全通信必須 |
| ファイル本体（At Rest） | クライアントサイド AES-256-GCM | アップロード前に暗号化 |
| メタデータ（At Rest） | SSE-S3（AWSマネージド）| 平文でAPI経由アクセス可 |
| 鍵管理 | ユーザーマスターパスワードからの派生 | クラウドに鍵を保存しない |

### 9.2 鍵導出フロー

```
MasterPassword (ユーザー入力)
  │
  ├─ Argon2id ─→ MasterKey (256bit)
  │                │
  │                ├─ HKDF(salt=file_specific_salt) ─→ FileEncryptionKey (256bit)
  │                │
  │                └─ 保存先: iPad Keychain / PC OS Credential Store
  │
  └─ 平文でクラウドに送信することは一切ない
```

### 9.3 ファイル暗号化フォーマット

```
[暗号化ファイルの構造]
┌─────────────────────────────────────────┐
│ Header (固定長)                         │
│   ├─ Magic Bytes: "SOLIDROP\x01" (8B)   │
│   ├─ Version: u8 (1B)                  │
│   ├─ Salt: [u8; 16] (16B)              │
│   ├─ Nonce: [u8; 12] (12B)             │
│   └─ Original Size: u64 LE (8B)        │
│                                         │
│ Encrypted Data (可変長)                  │
│   └─ AES-256-GCM ciphertext + tag      │
└─────────────────────────────────────────┘
```

### 9.4 鍵喪失リスク

**重要:** マスターパスワードを忘れるとクラウド上のデータが全喪失する。回復手段はない。

**運用対策（ユーザー責任）:**
- マスターパスワードを紙に書いて物理的に安全な場所に保管
- パスワードマネージャーへのバックアップ

---

## 10. 通信設計

### 10.1 Phase 1（MVP）

- 全通信: HTTPS (TLS 1.3)
- ファイル転送: 署名付きURL方式（クライアント ↔ S3直接）
- API通信: クライアント ↔ VPS上のRustサーバー
- 署名付きURLの有効期限: 3600秒（1時間）

### 10.2 同期モデル（Phase 1）

**単方向バックアップ + 手動復元。双方向リアルタイム同期はやらない。**

- **アップロード:** クライアントがSHA-256ハッシュをサーバーに送信→既存と一致すればスキップ（dedup）→不一致なら署名付きURL取得→S3にPUT
- **ダウンロード:** ユーザーがファイル一覧から選択→署名付きURL取得→S3からGET→復号→保存
- **コンフリクト:** 単一ユーザーのため原則発生しない。保険として楽観的ロック（ETag/Last-Modified比較）を導入

### 10.3 Phase 2以降で検討する通信トピック

以下は学習目的としても価値が高いが、Phase 1では実装しない:

- 差分同期（rsyncアルゴリズム、Content-Defined Chunking）
- 双方向同期の競合解決（ベクタークロック、CRDT）
- S3マルチパートアップロード（大容量ファイル対応）
- LAN内P2P転送（mDNS + ローカルHTTP）

---

## 11. データモデル

### 11.1 クラウド側（S3メタデータ）

S3オブジェクトのユーザーメタデータタグとして保存:

| タグキー | 値 | 例 |
|---|---|---|
| `x-amz-meta-content-hash` | 平文のSHA-256ハッシュ | `sha256:a1b2c3...` |
| `x-amz-meta-original-size` | 暗号化前のファイルサイズ(bytes) | `31457280` |
| `x-amz-meta-original-name` | 元のファイル名 | `illustration-01.clip` |

### 11.2 クライアント側（ローカルSQLite）

iPadアプリ内で管理するキャッシュ状態DB:

```sql
CREATE TABLE file_cache (
    path            TEXT PRIMARY KEY,  -- S3上のパス
    content_hash    TEXT NOT NULL,     -- 平文のSHA-256
    size_bytes      INTEGER NOT NULL,  -- 暗号化前のサイズ
    last_used       TIMESTAMP,         -- 最後にクリスタ等で開いた日時
    location        TEXT NOT NULL       -- 'local_and_cloud' | 'cloud_only'
                    CHECK (location IN ('local_and_cloud', 'cloud_only')),
    uploaded_at     TIMESTAMP,         -- S3にアップロードした日時
    local_path      TEXT               -- ローカルファイルパス（cloud_onlyならNULL）
);

CREATE INDEX idx_last_used ON file_cache(last_used);
CREATE INDEX idx_location ON file_cache(location);
```

### 11.3 PC CLI側

設定ファイル（TOML）:

```toml
[server]
endpoint = "https://your-vps-domain.com/api/v1"
api_key_env = "SOLIDROP_API_KEY"  # 環境変数名

[storage]
download_dir = "~/Art/synced"
upload_dir = "~/Art/to-upload"

[crypto]
# マスターキーの取得先（OS credential store）
keychain_service = "solidrop"
keychain_account = "master-key"
```

---

## 12. キャッシュ戦略（ストレージオフロード）

### 12.1 方針

コンピュータアーキテクチャのLRU（Least Recently Used）キャッシュ戦略を適用。iPadのローカルストレージをキャッシュ、S3をバッキングストアとみなす。

### 12.2 パラメータ

| パラメータ | デフォルト値 | 設定可能 |
|---|---|---|
| ローカル保持上限 | 60GB | ユーザー設定可 |
| バックアップ頻度 | 日次 | BGTaskSchedulerのOS制約に依存 |
| 退避方式 | 承認制（候補提示→ユーザー承認） | Phase 2で自動化検討 |

### 12.3 退避ロジック

```
[日次バックアップ時に実行]
1. ローカルの管理対象ファイルの合計サイズを算出
2. 上限（デフォルト60GB）を超過しているか判定
3. 超過している場合:
   a. file_cacheテーブルからlocation='local_and_cloud'のファイルを
      last_used昇順（古い順）で取得
   b. 超過分を解消するのに必要なファイルを退避候補として選出
   c. ユーザーに退避候補リストを通知（アプリ内通知）
   d. ユーザー承認後:
      - S3上にファイルが存在することを確認（content_hash照合）
      - ローカルファイルを削除
      - file_cacheのlocationを'cloud_only'に更新
4. 超過していない場合:
   - 未バックアップファイルのみS3にアップロード
```

### 12.4 承認制を選択した理由

完全自動退避は「今日描こうと思っていたファイルが消えていた」リスクがあり、お絵かきUXを最優先とする本プロジェクトの目的に反する。MVPでは承認制とし、使用感をもとにPhase 2で自動化を検討する。

---

## 13. 復元フロー

### 13.1 Phase 1（MVP）

1. アプリ内でクラウドファイル一覧を表示（`GET /api/v1/files`）
2. 復元したいファイルを選択
3. 署名付きダウンロードURLを取得（`POST /api/v1/presign/download`）
4. S3から暗号文をダウンロード
5. AES-256-GCM復号 + SHA-256ハッシュ検証
6. Files.appアクセス可能領域に保存
7. `file_cache` テーブルの `location` を `local_and_cloud` に更新

### 13.2 Phase 2以降

- S3バージョニングを活用し、同一パスの過去バージョン一覧を表示
- 特定バージョンを選択して復元（タイムスタンプで識別）
- バージョン保持ポリシー（例: 最新5世代 or 30日間）の設定

### 13.3 設計上の注意

S3バージョニングはPhase 1から有効にしておく。バージョニング自体のストレージコストは「旧バージョンのオブジェクトが追加で保存される分」だけであり、個人利用レベルでは微小。UI上の世代管理は後回しだが、データは保全される。

---

## 14. コスト試算

### 14.1 前提データ

| 項目 | 値 | 根拠 |
|---|---|---|
| .clipファイル中央値 | 30MB | ユーザー実測 |
| .clipファイル最大値 | 55MB | モノクロ漫画データ |
| 月間制作数（通常月） | 5枚 | ユーザー見積もり |
| 繁忙期 | 年2回、通常の3〜5倍 | 原稿期間・ゾーン突入時 |
| 資料画像 | 5MB × 30回/月 | ユーザー見積もり |
| 完成tiff | 10MB × 10回/月 | ユーザー見積もり |

### 14.2 月間データ量

| 項目 | 通常月 | 繁忙月 |
|---|---|---|
| .clipバックアップ | 150MB (30MB×5) | 450〜750MB |
| 資料画像送受信 | 150MB (5MB×30) | 同程度 |
| 完成tiff送受信 | 100MB (10MB×10) | 同程度 |
| **合計アップロード** | **約400MB** | **約1〜2GB** |

### 14.3 蓄積データ量予測

| 時点 | 蓄積量 |
|---|---|
| 1年後 | 約5〜10GB |
| 3年後 | 約15〜30GB |

### 14.4 AWSコスト試算（東京リージョン、ap-northeast-1）

**3年運用・30GB蓄積時点:**

| 項目 | 月額 |
|---|---|
| S3 Standard ストレージ (30GB) | $0.75 ($0.025/GB) |
| S3 Glacier Instant Retrieval (/archived/) | 適用後さらに低下 |
| S3 PUT/GETリクエスト (月数百回) | $0.01未満 |
| データ転送 Egress (PC DL 1GB/月) | $0.11 ($0.114/GB) |
| Lambda/Fargate | 不使用（VPS代用） |
| **合計** | **約$0.9/月 (約140円)** |

**初年度 (10GB時点):**

| 項目 | 月額 |
|---|---|
| S3 Standard ストレージ (10GB) | $0.25 |
| S3 リクエスト + Egress | $0.15 |
| **合計** | **約$0.4/月 (約60円)** |

### 14.5 VPSコスト

XServer VPSは既存契約のため追加コスト0。将来Fargateに移行する場合は月$5〜10程度を見込む。

### 14.6 NASとの比較

| 項目 | クラウド（本構成） | NAS |
|---|---|---|
| 初期コスト | $0 | 3〜4万円（HDD込み） |
| ランニングコスト | $0.4〜1/月 | 電気代 月500〜1000円 |
| Egress料金 | あり（微小） | なし（LAN内） |
| 外出先アクセス | 可能 | VPN設定が必要 |
| 冗長性 | S3が99.999999999%耐久性 | RAID構成に依存 |
| 学習価値 | AWS, Terraform, S3互換API | ネットワーク, Linux管理 |

本プロジェクトの目的（クラウド/システム設計の学習 + 低初期コスト）にはクラウドが適合。

---

## 15. 技術スタック

### 15.1 確定

| コンポーネント | 技術 | 選定理由 |
|---|---|---|
| APIサーバー | Rust (axum) | 学習第一優先 |
| PC CLIツール | Rust | サーバーとcrateを共有 |
| 暗号化ライブラリ | Rust (共通crate) | クライアント・サーバー共有 |
| iPad/Androidアプリ | Flutter | クロスプラットフォーム |
| クラウドストレージ | AWS S3 | 学習目的でAWS選択 |
| IaC | Terraform | AWSリソース管理 |
| APIサーバーデプロイ | Docker on XServer VPS | 既存契約活用、追加コスト0 |
| ローカルDB | SQLite | クライアント側キャッシュ管理 |

### 15.2 Rustクレート（サーバー/CLI）

```toml
# API Server
axum = "0.7"
aws-sdk-s3 = "1"
aws-config = "1"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tower-http = { version = "0.5", features = ["cors", "trace"] }
tracing = "0.1"
tracing-subscriber = "0.3"

# 暗号化（共通crate）
aes-gcm = "0.10"
argon2 = "0.5"
hkdf = "0.12"
sha2 = "0.10"
rand = "0.8"
```

### 15.3 将来の技術的負債・移行パス

| 現状 | 移行先 | トリガー |
|---|---|---|
| XServer VPS | AWS Fargate / Lambda | VPSリソース不足、IAMロール使用要望 |
| 手動ファイル選択 | File Provider Extension (Swift) | UX改善要望が高まった時 |
| 単方向バックアップ | 双方向同期 | PC編集→iPad反映の需要増 |
| Bearer Token認証 | OAuth2 / JWT | 複数デバイス管理が複雑化した時（可能性低） |

---

## 16. プロジェクト構成

```
project-root/
├── crates/
│   ├── crypto/            # 暗号化共通ライブラリ
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── encrypt.rs
│   │   │   ├── decrypt.rs
│   │   │   ├── key_derivation.rs
│   │   │   └── hash.rs
│   │   └── Cargo.toml
│   ├── api-server/        # axum HTTPサーバー
│   │   ├── src/
│   │   │   ├── main.rs
│   │   │   ├── routes/
│   │   │   ├── s3_client.rs
│   │   │   └── config.rs
│   │   ├── Cargo.toml
│   │   └── Dockerfile
│   └── cli/               # PC用CLIツール
│       ├── src/
│       │   ├── main.rs
│       │   ├── commands/
│       │   └── config.rs
│       └── Cargo.toml
├── flutter/
│   └── solidrop/          # iPad/Androidアプリ
│       ├── lib/
│       ├── pubspec.yaml
│       └── ...
├── infra/
│   └── terraform/         # AWSリソース定義
│       ├── main.tf
│       ├── s3.tf
│       ├── iam.tf
│       └── variables.tf
├── docs/
│   ├── design/
│   │   ├── architecture.md
│   │   ├── data-model.md
│   │   └── user-flow.md
│   ├── progress/
│   ｜   ├── 00-init-stub-plan.md     # 実装計画書
│   ｜   ├── 00-init-stub-report.md   # 実装報告書/引継書
│   ｜   └── lessons.md   # 実装時のトラブルシューティング記録
│   └── api-spec.yaml      # OpenAPI仕様（将来）
├── Cargo.toml             # Cargo workspace
├── docker-compose.yml
└── README.md # 本ドキュメント
```

### 16.1 Cargo ワークスペース

```toml
# project-root/Cargo.toml
[workspace]
members = [
    "crates/crypto",
    "crates/api-server",
    "crates/cli",
]
```

暗号化処理はサーバーとCLI双方で使用するため、共通crateとして切り出す。Rustのモジュール設計・ワークスペース運用を学ぶ良い題材である。

---

## 17. 開発フェーズ

### Phase 0: 検証・基盤構築（2〜3週間）

| タスク | 成果物 |
|---|---|
| Terraform で S3バケット + IAMユーザー作成 | `infra/terraform/` |
| `crates/crypto/` の実装 + ユニットテスト | AES-256-GCM暗号化・復号、Argon2id鍵導出、SHA-256ハッシュ |
| Rust で署名付きURL発行の動作確認（最小コード） | 手動curlでS3にPUT/GETできることの確認 |
| iPad実機でクリスタの保存挙動を確認 | Files.app経由での読み書き可否、保存先の確認結果ドキュメント |

### Phase 1: MVP（1〜2ヶ月）

| タスク | 成果物 |
|---|---|
| `crates/api-server/` 全エンドポイント実装 | Docker化、VPSデプロイ |
| `crates/cli/` 実装 | PC からアップロード/ダウンロード/一覧 |
| `flutter/solidrop/` 実装 | iPad アプリ（ファイル選択、アップロード、一覧、ダウンロード） |
| キャッシュ管理（ローカルSQLite + 退避候補提示） | ストレージオフロード機能 |
| 日次自動バックアップ（BGTaskScheduler） | バックグラウンドバックアップ |

**Phase 1完了時に実現すること:**
- iPadの容量確保（不要ファイルのクラウド退避）
- iPadからPCへの暗号化ファイル転送
- PCからiPadへのファイル送信

### Phase 2: キャッシュ管理高度化 + PC連携強化

| タスク | 成果物 |
|---|---|
| S3バージョニングUIでの世代管理・復元 | 過去バージョン閲覧・復元機能 |
| 退避の自動化（使用パターン学習後） | 手動承認不要の自動退避 |
| Fargate / Lambda への段階的移行検討 | コスト・運用の最適化 |
| PC用GUIアプリ（Rust + GUI FW） | CLIからの脱却 |

### Phase 3: UX向上

| タスク | 成果物 |
|---|---|
| File Provider Extension (Swift) | クリスタからの直接読み書き |
| LAN内P2P転送 (mDNS + ローカルHTTP) | AirDrop的体験 |
| Android アプリ | 資料閲覧・アップロード |

---

## 18. 未決定事項・保留事項

以下は議論の過程で明示的に判断を保留した項目、または詳細が未整理の項目である。実装時に順次決定する。

### 18.1 未決定（実装前に決定が必要）

| ID | 項目 | 状態 | 備考 |
|---|---|---|---|
| TBD-1 | S3バケット名の正式名称 | 未決定 | `{project-name}-art-storage` のプレースホルダー |
| TBD-2 | VPSドメイン名 / TLS証明書の取得方法 | 未決定 | Let's Encrypt or Caddy |
| TBD-3 | APIキーの生成・管理方法 | 未決定 | 初回セットアップ時に生成、環境変数 or ファイル管理 |
| TBD-4 | IAMアクセスキーのローテーション頻度 | 未決定 | 90日ごと程度を推奨 |
| TBD-5 | Argon2idのパラメータ（メモリコスト、反復回数） | 未決定 | OWASP推奨値を基準に、クライアント端末の性能で調整 |
| TBD-6 | BGTaskSchedulerの具体的なタスク識別子・実装方式 | 未決定 | Flutter側プラグイン選定にも依存 |
| TBD-7 | Flutter側のS3直接アップロード実装方式 | 未決定 | Dart HTTPクライアント or platform channel経由 |
| TBD-8 | Flutterの暗号化処理: Dart実装 or Rust FFI | 未決定 | パフォーマンス検証後に判断。Rust FFIなら暗号化crateを直接利用可 |

### 18.2 保留（Phase 2以降で検討）

| ID | 項目 | 保留理由 |
|---|---|---|
| DEF-1 | File Provider Extension の詳細設計 | Phase 3。実装難易度が高く、MVPの知見蓄積後に着手 |
| DEF-2 | LAN内P2P転送の技術選定 | Phase 3。YAGNI。クラウド経由で不満が出てから検討 |
| DEF-3 | S3バージョニングの世代管理UI | Phase 2。バージョニング自体はPhase 1から有効化済み |
| DEF-4 | 退避の完全自動化 | Phase 2。承認制の使用感を見てから判断 |
| DEF-5 | 差分同期（rsync的アルゴリズム） | Phase 2以降。.clipファイルのバイナリ構造の特性調査が前提 |
| DEF-6 | 双方向同期のコンフリクト解決 | Phase 2以降。現時点では単方向で十分 |
| DEF-7 | Fargate / Lambda 移行の具体的トリガー条件 | VPS運用の実績に基づき判断 |
| DEF-8 | Webフロントエンド（Ruby） | スコープ外。必要性が生じた場合に検討 |
| DEF-9 | Procreate / FreeForm ファイルの扱い | 議論未着手。.clipと同様の扱いが可能かは調査が必要 |

### 18.3 リスク

| ID | リスク | 影響 | 対策 |
|---|---|---|---|
| RISK-1 | マスターパスワード喪失 | クラウド上の全データ復号不可 | ユーザー運用（物理的保管、パスワードマネージャー） |
| RISK-2 | VPS上のIAMキー漏洩 | S3バケットへの不正アクセス | 最小権限IAM、アクセスキーローテーション、VPSセキュリティ強化 |
| RISK-3 | iOSのバックグラウンド実行制約 | 日次バックアップの実行頻度・タイミングが保証されない | ユーザーへの説明。手動バックアップも併用できるUI |
| RISK-4 | クリスタのファイルI/O挙動が想定と異なる | ファイル監視・取得のロジック見直し | Phase 0の実機調査で早期に検証 |
| RISK-5 | 暗号化処理のパフォーマンス（大容量ファイル） | アップロード時間の増大 | プロファイリング後にストリーミング暗号化等を検討 |

---

> **本ドキュメントは開発進行に伴い更新される。各Phaseの完了時にレビューと改訂を行うこと。**
