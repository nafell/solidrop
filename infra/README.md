# Solidrop インフラ セットアップガイド

XServer VPS 上で Solidrop API サーバーをステージング / 本番環境としてデプロイするための完全手順。

---

## 前提条件

| 要件 | 備考 |
|---|---|
| AWS アカウント | S3 バケット・IAM ユーザー作成に必要 |
| Terraform ≥ 1.0 | `infra/terraform/` のプロビジョニングに使用 |
| XServer VPS | 既存契約。Docker・systemd が動作していること |
| Caddy コンテナ | VPS 上で既に稼働していること。`--network host` モードを推奨 |
| Docker (ローカル) | `deploy.sh` でイメージをビルドして VPS に転送する場合 |
| ドメインの DNS 管理権限 | A レコード追加に必要 |

---

## ディレクトリ構成

```
infra/
├── terraform/               # AWS S3 / IAM — Terraform 設定
│   ├── main.tf
│   ├── s3.tf
│   ├── iam.tf
│   ├── variables.tf
│   ├── staging.tfvars       # ステージング用変数
│   ├── prod.tfvars          # 本番用変数
│   └── SPEC.md              # 詳細仕様・設計根拠
└── vps/                     # VPS 設定・デプロイスクリプト
    ├── Caddyfile.staging.snippet    # Caddy に追加するステージングブロック
    ├── caddy-apply.sh               # Caddyfile への snippet 適用スクリプト
    ├── solidrop-api-staging.service # systemd サービスファイル (staging)
    ├── deploy.sh                    # ローカル PC → VPS デプロイ
    └── deploy-inside-vps.sh         # VPS 内直接デプロイ
```

---

## Step 1: AWS インフラを Terraform でプロビジョニング

```bash
cd infra/terraform

# 初回のみ
terraform init

# ステージング
terraform plan -var-file=staging.tfvars
terraform apply -var-file=staging.tfvars

# 本番
terraform plan -var-file=prod.tfvars
terraform apply -var-file=prod.tfvars
```

作成されるリソース:

| リソース | staging | prod |
|---|---|---|
| S3 バケット | `nafell-solidrop-staging` | `nafell-solidrop-storage` |
| IAM ユーザー | `solidrop-api-staging` | `solidrop-api-prod` |
| IAM ポリシー | バケットスコープの最小権限 | 同上 |

Terraform 適用後、AWS コンソールまたは CLI で IAM ユーザーのアクセスキーを発行し、手元に保管する。

---

## Step 2: DNS A レコード追加

> **重要: Caddy 設定を適用する前に DNS を設定すること。** DNS 伝播前に Caddy を起動すると Let's Encrypt の ACME チャレンジが失敗し、証明書取得に失敗する。

ドメインの DNS 管理画面で以下を追加する:

| Type | Name | Value | TTL |
|---|---|---|---|
| A | `staging.api.solidrop.nafell.dev` | `<VPS の IPv4>` | 300 |
| A | `api.solidrop.nafell.dev` | `<VPS の IPv4>` | 300 (prod 追加時) |

伝播確認:

```bash
dig A staging.api.solidrop.nafell.dev @1.1.1.1
# ANSWER セクションに VPS の IP が返ること
```

TTL が 300 秒のため、通常は数分以内に伝播する。

---

## Step 3: VPS に環境変数ファイルを作成

VPS に SSH でログインして実行する。

```bash
# API トークン検証用ハッシュを生成 (ローカル PC で実行)
cargo run -p solidrop-cli -- print-verifier
# → SOLIDROP_API_KEY_VERIFIER_SHA256=<hash>

# VPS で実行
sudo mkdir -p /etc/solidrop
sudo tee /etc/solidrop/staging.env > /dev/null <<EOF
AWS_ACCESS_KEY_ID=<Step 1 で発行したアクセスキー ID>
AWS_SECRET_ACCESS_KEY=<シークレットアクセスキー>
SOLIDROP_API_KEY_VERIFIER_SHA256=<print-verifier で生成したハッシュ>
EOF
sudo chmod 600 /etc/solidrop/staging.env
```

本番環境の場合は `/etc/solidrop/prod.env` に同様の内容を作成する。

---

## Step 4: systemd サービスを配置・有効化

```bash
# リポジトリを VPS にクローンしている場合 (推奨)
sudo cp /opt/solidrop/infra/vps/solidrop-api-staging.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable solidrop-api-staging
```

サービスファイルの主要設定:

- **バインドアドレス**: `127.0.0.1:3001` (ローカルループバックのみ。Caddy 経由でのみアクセス)
- **環境変数**: `/etc/solidrop/staging.env` から読み込み
- **自動再起動**: クラッシュ時 5 秒後に再起動

---

## Step 5: Docker イメージをビルドして起動

### 方法 A: ローカル PC からビルド・転送 (推奨: VPS に Rust 環境が不要)

```bash
# リポジトリルートで実行
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh staging
```

内部処理:
1. `crates/api-server/Dockerfile` を使用してローカルでビルド
2. `docker save | ssh docker load` で VPS に転送
3. `systemctl restart solidrop-api-staging` でサービス再起動

### 方法 B: VPS 内でビルド (VPS に Rust / Docker が必要)

```bash
# VPS 上で実行。事前に /opt/solidrop に git clone が必要
REPO_DIR=/opt/solidrop bash infra/vps/deploy-inside-vps.sh staging
```

---

## Step 6: Caddy リバースプロキシ設定を適用

DNS 伝播が確認できてから実行する。

```bash
# VPS 上で実行
REPO_DIR=/opt/solidrop bash infra/vps/caddy-apply.sh staging
```

スクリプトは以下を idempotent に実行する:
1. `docker inspect` で Caddyfile のホスト側マウントパスを自動検出
2. `staging.api.solidrop.nafell.dev` ブロックが未追記なら追記
3. `caddy reload` でゼロダウンタイム適用・TLS 証明書取得開始

Caddy が Let's Encrypt から TLS 証明書を自動取得する (数十秒〜数分)。

---

## Step 7: 動作確認

```bash
# HTTP → HTTPS リダイレクト
curl -v http://staging.api.solidrop.nafell.dev/
# → 308 Permanent Redirect

# HTTPS ヘルスチェック
curl https://staging.api.solidrop.nafell.dev/health
# → {"status":"ok"}
```

---

## 更新デプロイ手順

コード変更後の再デプロイは Step 5 のみ繰り返す:

```bash
# ローカル PC から
VPS_HOST=user@<VPS_IP> bash infra/vps/deploy.sh staging
```

---

## トラブルシューティング

### TLS: `curl: (35) TLS connect error` / NXDOMAIN

**原因**: DNS 伝播前に Caddy が ACME チャレンジを試みて失敗し、結果をキャッシュした。

**解決手順**:

```bash
# 1. DNS 伝播を確認
dig A staging.api.solidrop.nafell.dev @1.1.1.1

# 2. Caddy がキャッシュした失敗済み ACME データを削除
docker exec caddy sh -c "rm -rf \
  /data/caddy/certificates/acme-v02.api.letsencrypt.org-directory/staging.api.solidrop.nafell.dev \
  /data/caddy/certificates/acme.zerossl.com-v2-DV90/staging.api.solidrop.nafell.dev"

# 3. Caddy を再起動
docker restart caddy

# 4. ログで証明書取得を監視
docker logs caddy -f 2>&1 | grep staging.api.solidrop
```

### API サーバーが起動しない

```bash
# サービス状態確認
sudo systemctl status solidrop-api-staging

# ログ確認
journalctl -u solidrop-api-staging -n 50
docker logs solidrop-api-staging
```

主な原因:
- `/etc/solidrop/staging.env` が存在しない・権限が正しくない
- Docker イメージ `solidrop-api-server:staging` が存在しない (Step 5 が未完了)

### Caddy が Caddyfile マウントを見つけられない

```bash
# マウント状況を確認
docker inspect caddy --format '{{json .Mounts}}'
```

`/etc/caddy/Caddyfile` が bind-mount されていない場合、`caddy-apply.sh` は動作しない。Caddy コンテナの起動コマンドを確認し、適切なボリュームマウントを追加すること。

### Let's Encrypt レートリミット

1 週間に同一ドメインで発行できる証明書は 5 枚まで。失敗を繰り返すと上限に達する場合がある。以下で確認:

```
https://tools.letsdebug.net/cert-search?m=domain&q=staging.api.solidrop.nafell.dev
```

レートリミットに達した場合、Let's Encrypt Staging CA を一時的に使用するか、7 日間待つ。

---

## 本番環境 (prod) の追加手順

staging と同じ手順を以下の差分で実施する:

| 項目 | staging | prod |
|---|---|---|
| ドメイン | `staging.api.solidrop.nafell.dev` | `api.solidrop.nafell.dev` |
| ポート (コンテナ内部) | `127.0.0.1:3001:3000` | `127.0.0.1:3002:3000` |
| S3 バケット | `nafell-solidrop-staging` | `nafell-solidrop-storage` |
| 環境変数ファイル | `/etc/solidrop/staging.env` | `/etc/solidrop/prod.env` |
| systemd サービス | `solidrop-api-staging` | `solidrop-api-prod` |
| Docker イメージタグ | `solidrop-api-server:staging` | `solidrop-api-server:prod` |
| Caddyfile snippet | `Caddyfile.staging.snippet` | `Caddyfile.prod.snippet` (要作成) |

`infra/vps/Caddyfile.staging.snippet` を参考に `Caddyfile.prod.snippet` を作成し、同様に `caddy-apply.sh prod` で適用する。

---

## セキュリティ注意事項

- `/etc/solidrop/*.env` は `chmod 600` で root のみ読み取り可能にする
- IAM アクセスキーは定期的にローテーションすること (目安: 90 日)
- VPS への SSH は公開鍵認証のみ許可し、パスワード認証を無効化すること
- `solidrop-api-staging.service` はローカルループバック (`127.0.0.1`) にのみバインドする設計になっており、Caddy 経由でのみ外部公開される
