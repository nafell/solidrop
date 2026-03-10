# Web 画像ビューア実装レポート

**日付:** 2026-03-11
**対応ブランチ:** feat/webui
**ステータス:** 実装完了・ビルド確認済み

---

## 概要

`/files` ルートのテキスト一覧に加え、暗号化ファイルをクライアントサイドで復号してサムネイルグリッド表示・ライトボックス閲覧できる `/viewer` ルートを追加した。
既存の `decryptFile` + presign フローを再利用し、Blob URL をモジュールレベルキャッシュで管理することで再マウント時の再フェッチを排除している。

---

## 実装したファイル一覧

### 新規ファイル

| ファイル | 役割 |
|---|---|
| `web/src/utils/format.ts` | `formatBytes` / `formatDate` を `FileList.tsx` から分離して共有ユーティリティ化 |
| `web/src/hooks/useThumbnail.ts` | presign → S3 GET → 復号 → Blob URL 生成フック。モジュールレベルキャッシュ付き。`isImageFile` もエクスポート |
| `web/src/components/ThumbnailCard.tsx` | 単一サムネイルカード。`IntersectionObserver` による遅延ロード・スピナー・エラー表示・`<img>` |
| `web/src/components/ThumbnailGrid.tsx` | `FileEntry[]` を画像ファイルに絞り込み、`ThumbnailCard` をグリッド配置 |
| `web/src/components/Lightbox.tsx` | フルサイズ画像モーダル。Prev/Next ボタン・Escape/ArrowLeft/ArrowRight キーボード操作・ファイル名・サイズ表示 |
| `web/src/pages/ViewerPage.tsx` | `/viewer` ルートページ。ヘッダー + nav + `ThumbnailGrid` + `Lightbox` |

### 変更ファイル

| ファイル | 変更内容 |
|---|---|
| `web/src/router.tsx` | `viewerRoute` (`/viewer`) を追加、`routeTree` に追加 |
| `web/src/pages/FilesPage.tsx` | `<nav>` を追加（一覧・ビューア リンク） |
| `web/src/components/FileList.tsx` | `formatBytes` / `formatDate` をローカル定義から `utils/format.ts` の import に変更 |
| `web/src/index.css` | `.app-nav`・`.thumbnail-*`・`.lightbox-*`・スピナーアニメーション CSS を追加 |

---

## 設計上の判断

### 1. モジュールレベル Blob URL キャッシュ

```ts
const blobUrlCache = new Map<string, string>() // path → blob URL
```

React コンポーネントライフサイクルの外（モジュールスコープ）に置くことで、`ThumbnailCard` のアンマウント・再マウントをまたいでキャッシュが保持される。`/files` ↔ `/viewer` 間を行き来しても presign / S3 GET / 復号を再実行しない。

`URL.createObjectURL` で確保した Blob URL は `URL.revokeObjectURL` を呼ばない（タブが閉じられるまで保持）。タブ 1 セッション内の画像数は限られており、メモリ量も問題にならないと判断した。

### 2. IntersectionObserver による遅延ロード

`ThumbnailCard` は `rootMargin: '200px'` のマージンでビューポート外の事前ロードを行う。`enabled` フラグが `false` の間は `useThumbnail` が `'idle'` 状態を維持し、presign API も呼ばない。ページネーション実装前でファイル数が多い場合のリクエスト爆発を防ぐ。

### 3. Lightbox に `useThumbnail(path, true)` を直接渡す

Lightbox は `enabled=true` 固定で `useThumbnail` を呼ぶ。サムネイルグリッドで既にロード済みのファイルはキャッシュヒットで即座に表示される。未ロード画像をライトボックスで直接開いた場合（ディープリンク等）もロードできる。

### 4. `isImageFile` の配置

画像判定ロジック（拡張子チェック）は `ThumbnailGrid` と `ViewerPage` の両方から使われるため、`useThumbnail.ts` に同居させてエクスポートした。単独ユーティリティファイルを作るほどの規模ではないと判断。

### 5. `formatBytes` / `formatDate` の抽出

`FileList.tsx` にローカル定義されていた 2 関数を `Lightbox.tsx`（`formatBytes` のみ使用）でも必要になったため `utils/format.ts` に移動。

---

## データフロー

```
/viewer マウント
  └─ useFiles() [TanStack Query — FilesPage と共通キャッシュ]
       └─ FileEntry[] → isImageFile() でフィルタ → imageFiles[]
            └─ ThumbnailCard × N
                 └─ IntersectionObserver: ビューポート侵入で enabled=true
                      └─ useThumbnail(path, enabled)
                           ├─ キャッシュヒット → 即 blobUrl 返却
                           └─ キャッシュミス → presign → S3 GET → 復号 → blobUrl

サムネイルクリック → setLightboxIndex(i)
  └─ Lightbox: blobUrl はキャッシュ済みのため即表示
       ├─ ← / → or Prev/Next → navigate
       └─ Escape or X → close
```

---

## ビルド成果物

```
dist/index.html                   0.39 kB │ gzip:   0.26 kB
dist/assets/index-*.css           5.05 kB │ gzip:   1.59 kB
dist/assets/index-*.js          350.49 kB │ gzip: 111.17 kB
```

CSS が約 +2KB 増加（サムネイルグリッド・ライトボックスのスタイル）。JS バンドルサイズはほぼ変わらず（新規依存なし）。

---

## 残課題・今後の検討

| 項目 | 備考 |
|---|---|
| 非画像ファイルのプレビュー | PDF・テキスト等は未対応。将来的に拡張子別の Preview コンポーネントが必要 |
| Blob URL のメモリ管理 | 現状はタブ閉鎖まで保持。画像数が増えた場合は LRU で revoke する仕組みが必要になる可能性 |
| ページネーション対応 | `useFiles` が全件取得のため、ファイル数が多い場合は無限スクロールとの組み合わせが必要 |
| ドラッグ操作によるライトボックス画像切り替え | スワイプ/ドラッグ未対応（キーボード・ボタンのみ） |
