# Web フロントエンド インフラ設定手順

フロントエンド開発を始める前に、VPS 側で必要なインフラを整備する手順書。
API インフラ（`infra/vps/` 以下）の設定済みを前提とする。

---

## ポート割り当て（全体）

| コンテナ | ホスト側ポート | コンテナ側ポート | 環境 |
|---|---|---|---|
| `solidrop-api-staging` | 3001 | 3000 | staging |
| `solidrop-api-prod`    | 3002 | 3000 | prod |
| `solidrop-web-staging` | 3011 | 80   | staging |
| `solidrop-web-prod`    | 3012 | 80   | prod |

---

## 前提条件（確認事項）

VPS 上で以下が完了していること:

- [ ] Docker がインストール済み・起動中
- [ ] リポジトリが `/opt/solidrop` にクローン済み
- [ ] Caddy コンテナが起動中（`docker ps` で `caddy` が表示される）
- [ ] API staging が動作中（`curl https://staging.api.solidrop.nafell.dev/health` が 200 を返す）
- [ ] DNS: `staging.web.solidrop.nafell.dev` が VPS の IP に向いている
- [ ] DNS: `web.solidrop.nafell.dev` が VPS の IP に向いている

---

## 1. Staging の初回セットアップ

### 1-1. systemd サービスファイルを配置

```bash
sudo cp /opt/solidrop/infra/vps/solidrop-web-staging.service \
        /etc/systemd/system/solidrop-web-staging.service

sudo systemctl daemon-reload
sudo systemctl enable solidrop-web-staging
```

### 1-2. 初回 Docker イメージをビルドして起動

**PC から転送する場合（ローカルビルド）:**

```bash
# リポジトリルートで実行
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh staging web
```

**VPS 上で直接ビルドする場合:**

```bash
# VPS 上で実行
bash /opt/solidrop/infra/vps/deploy-inside-vps.sh staging web
```

> **注意:** `web/` ディレクトリに Vite プロジェクト（`package.json`, `src/` 等）が存在する状態で実行すること。
> フロントエンドのソースコードがまだない場合は、先に `web/` を作成してからデプロイする。

### 1-3. サービス起動を確認

```bash
sudo systemctl status solidrop-web-staging
# Active: active (running) と表示されること
```

### 1-4. Caddy に staging.web ドメインを追加

```bash
REPO_DIR=/opt/solidrop bash /opt/solidrop/infra/vps/caddy-apply.sh staging
```

このコマンドは以下の 2 つを Caddyfile に追記して Caddy をリロードする（冪等）:
- `Caddyfile.staging.snippet`（`staging.api.*`、既存）
- `Caddyfile.web-staging.snippet`（`staging.web.*`、新規）

### 1-5. 動作確認

```bash
# nginx コンテナに直接アクセス（Caddy を経由しない）
curl -I http://localhost:3011/

# Caddy 経由（HTTPS + ドメイン）
curl -I https://staging.web.solidrop.nafell.dev/

# API パスが API コンテナに透過されること
curl https://staging.web.solidrop.nafell.dev/health
```

期待する結果:
- `localhost:3011/` → `200 OK`（nginx がインデックスを返す）
- `staging.web.solidrop.nafell.dev/` → `200 OK`（Let's Encrypt 証明書が自動取得される）
- `/health` → `{"status":"ok"}` など（API からの応答）

---

## 2. Production の初回セットアップ

> **前提:** API prod（`solidrop-api-prod` コンテナ、ポート 3002）が起動済みであること。
> API prod のセットアップは `infra/vps/` の API 手順書を参照。

### 2-1. systemd サービスファイルを配置

```bash
sudo cp /opt/solidrop/infra/vps/solidrop-web-prod.service \
        /etc/systemd/system/solidrop-web-prod.service

sudo systemctl daemon-reload
sudo systemctl enable solidrop-web-prod
```

### 2-2. 初回 Docker イメージをビルドして起動

```bash
# PC から転送する場合
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh prod web

# または VPS 上で直接ビルド
bash /opt/solidrop/infra/vps/deploy-inside-vps.sh prod web
```

### 2-3. Caddy に web (prod) ドメインを追加

```bash
REPO_DIR=/opt/solidrop bash /opt/solidrop/infra/vps/caddy-apply.sh prod
```

`Caddyfile.web-prod.snippet`（`web.solidrop.nafell.dev`）が追記される。

### 2-4. 動作確認

```bash
curl -I https://web.solidrop.nafell.dev/
curl https://web.solidrod.nafell.dev/health
```

---

## 3. 通常のデプロイ（コード更新後）

フロントエンドのコードを変更して再デプロイする際の手順。

### PC からデプロイ（推奨）

```bash
# staging のみ
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh staging web

# prod のみ
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh prod web

# staging + prod を両方
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh staging all
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh prod all
```

### VPS 上でデプロイ

```bash
bash /opt/solidrop/infra/vps/deploy-inside-vps.sh staging web
bash /opt/solidrop/infra/vps/deploy-inside-vps.sh prod web
```

デプロイスクリプトは以下を行う:
1. `git pull --ff-only`（VPS 上ビルドの場合のみ）
2. `docker build -f web/Dockerfile -t solidrop-web:<env> web/`
3. `sudo systemctl restart solidrop-web-<env>`

---

## 4. ローカル開発環境との対応

| 環境 | フロントエンド | API |
|---|---|---|
| ローカル dev | Vite dev server `:5173` | `localhost:3000`（docker-compose）|
| VPS staging | nginx コンテナ `:3011`（Caddy 経由） | axum コンテナ `:3001`（Caddy 経由）|
| VPS prod    | nginx コンテナ `:3012`（Caddy 経由） | axum コンテナ `:3002`（Caddy 経由）|

ローカル dev では Vite と API が別ポートになるため、`vite.config.ts` にプロキシ設定を追加する（CORS 回避):

```typescript
// vite.config.ts
export default defineConfig({
  server: {
    proxy: {
      '/api': 'http://localhost:3000',
      '/health': 'http://localhost:3000',
    },
  },
})
```

---

## 5. トラブルシューティング

### コンテナが起動しない

```bash
# サービスのログを確認
sudo journalctl -u solidrop-web-staging -n 50

# Docker ログを直接確認
docker logs solidrop-web-staging
```

### Caddy が 502 を返す

```bash
# nginx コンテナが起動しているか確認
docker ps | grep solidrop-web

# ポートが開いているか確認
ss -tlnp | grep 3011
```

### Let's Encrypt の証明書が取得できない

```bash
# Caddy のログを確認
docker logs caddy --tail 50
```

DNS が VPS IP に向いているか確認（証明書取得には DNS 伝播が必要）。

### Caddy リロード後もドメインが反映されない

```bash
# 手動でリロード
docker exec caddy caddy reload --config /etc/caddy/Caddyfile

# Caddyfile の現在の内容を確認
docker inspect caddy --format '{{range .Mounts}}{{if eq .Destination "/etc/caddy/Caddyfile"}}{{.Source}}{{end}}{{end}}' \
  | xargs cat
```

---

## 6. ファイル構成リファレンス

```
infra/
├── nginx/
│   └── default.conf                    # nginx SPA フォールバック設定
└── vps/
    ├── solidrop-web-staging.service     # systemd unit (staging, port 3011)
    ├── solidrop-web-prod.service        # systemd unit (prod, port 3012)
    ├── Caddyfile.web-staging.snippet    # Caddy: staging.web.solidrop.nafell.dev
    ├── Caddyfile.web-prod.snippet       # Caddy: web.solidrop.nafell.dev
    ├── caddy-apply.sh                   # Caddyfile にスニペットを追記して reload
    ├── deploy.sh                        # PC → VPS へイメージ転送 + restart
    └── deploy-inside-vps.sh            # VPS 上でビルド + restart

web/
├── Dockerfile                          # node build + nginx serve (multi-stage)
└── web-infra-instructions.md           # 本ファイル
```
