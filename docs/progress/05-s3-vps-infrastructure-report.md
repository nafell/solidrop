# 05 — S3 + VPS インフラ構築レポート

このフェーズではステージング環境のインフラを完全に稼働させた。具体的には、Terraform による AWS S3 / IAM のプロビジョニング、XServer VPS 上の Caddy + systemd による API サーバーホスティング、そして HTTPS 証明書取得までを一通り実施した。

最終確認:

```
curl https://staging.api.solidrop.nafell.dev/health
{"status":"ok"}
```

---

## 実施内容サマリー

| フェーズ | 内容 | ステータス |
|---|---|---|
| 1 | Terraform — S3 バケット・IAM ユーザー作成 | 完了 |
| 2 | VPS — systemd サービスファイル配置・API コンテナ起動 | 完了 |
| 3 | VPS — Caddy へのリバースプロキシ設定適用 | 完了 |
| 4 | DNS — staging サブドメイン A レコード追加 | 完了 (手動) |
| 5 | TLS — Let's Encrypt 証明書取得 | 完了 (Caddy 自動) |

---

## フェーズ 1: Terraform (AWS S3 / IAM)

`infra/terraform/` 配下の設定を用いて以下を作成した。

- **S3 バケット** `nafell-solidrop-staging`（東京リージョン）
  - バージョニング有効
  - SSE-S3 暗号化
  - パブリックアクセス完全ブロック
  - `archived/` プレフィックスに 90日後 Glacier Instant Retrieval 移行ライフサイクル
- **IAM ユーザー** `solidrop-api-staging`
  - インラインポリシー: `s3:PutObject`, `s3:GetObject`, `s3:DeleteObject`, `s3:ListBucket` をステージングバケットのみに限定

```bash
cd infra/terraform
terraform init
terraform apply -var-file=staging.tfvars
```

Terraform 適用後、AWS コンソールまたは AWS CLI でアクセスキーを発行し `/etc/solidrop/staging.env` に設定する（後述）。

---

## フェーズ 2: VPS — API サーバー起動

### systemd サービス配置

`infra/vps/solidrop-api-staging.service` を VPS 上の `/etc/systemd/system/` に配置し有効化する。

```bash
sudo cp infra/vps/solidrop-api-staging.service /etc/systemd/system/
sudo systemctl daemon-reload
sudo systemctl enable solidrop-api-staging
```

### 環境変数ファイル作成

```bash
sudo mkdir -p /etc/solidrop
sudo tee /etc/solidrop/staging.env <<EOF
AWS_ACCESS_KEY_ID=<Terraform で発行した IAM アクセスキー>
AWS_SECRET_ACCESS_KEY=<シークレットキー>
SOLIDROP_API_KEY_VERIFIER_SHA256=<solidrop print-verifier で生成したハッシュ>
EOF
sudo chmod 600 /etc/solidrop/staging.env
```

`SOLIDROP_API_KEY_VERIFIER_SHA256` は CLI で生成する:

```bash
# ローカル PC で実行
cargo run -p solidrop-cli -- print-verifier
```

### Docker イメージビルド・デプロイ

**ローカル PC → VPS 転送方式** (`infra/vps/deploy.sh`):
```bash
VPS_HOST=user@162.43.47.174 bash infra/vps/deploy.sh staging
```

**VPS 内直接ビルド方式** (`infra/vps/deploy-inside-vps.sh`):
```bash
# VPS 上で実行
REPO_DIR=/opt/solidrop bash infra/vps/deploy-inside-vps.sh staging
```

---

## フェーズ 3: Caddy リバースプロキシ設定

既存の Caddy コンテナ (`*.is.lalafell.fun` 等を処理) に `staging.api.solidrop.nafell.dev` のブロックを追記する。

`infra/vps/caddy-apply.sh` が冪等に対応する:

```bash
# VPS 上で実行
REPO_DIR=/opt/solidrop bash infra/vps/caddy-apply.sh staging
```

スクリプトは:
1. `docker inspect` で Caddyfile のホスト側パスを自動検出
2. `staging.api.solidrop.nafell.dev` ブロックがなければ追記 (idempotent)
3. `caddy reload` でゼロダウンタイム適用

適用後の Caddyfile 追記内容 (`Caddyfile.staging.snippet`):
```
staging.api.solidrop.nafell.dev {
    reverse_proxy http://localhost:3001
}
```

---

## フェーズ 4: DNS A レコード追加

ドメインの DNS 設定画面（Cloudflare 等）で以下を追加した:

| Type | Name | Value | TTL |
|---|---|---|---|
| A | `staging.api.solidrop.nafell.dev` | `162.43.47.174` | 300 |

確認:
```bash
dig A staging.api.solidrop.nafell.dev @1.1.1.1
# → 162.43.47.174
```

---

## フェーズ 5: TLS 証明書 (Let's Encrypt)

Caddy が自動で Let's Encrypt 証明書を取得する。ただし **DNS 伝播前に Caddy を起動すると ACME チャレンジが NXDOMAIN で失敗し、証明書取得に失敗する**。

### 発生した問題

```
DNS problem: NXDOMAIN looking up A for staging.api.solidrop.nafell.dev
```

Caddy 起動時点では DNS レコードがまだ伝播していなかったため、Let's Encrypt の検証サーバーが NXDOMAIN を返した。Caddy はこの失敗をキャッシュするため、DNS が伝播した後も再起動しなければ再試行されない。

### 解決手順

1. DNS 伝播を確認する (`dig @1.1.1.1` で正引きできること)
2. Caddy がキャッシュした ACME データを削除する:
   ```bash
   docker exec caddy sh -c "rm -rf \
     /data/caddy/certificates/acme-v02.api.letsencrypt.org-directory/staging.api.solidrop.nafell.dev \
     /data/caddy/certificates/acme.zerossl.com-v2-DV90/staging.api.solidrop.nafell.dev"
   ```
3. Caddy を再起動する:
   ```bash
   docker restart caddy
   ```
4. ログで証明書取得を確認する:
   ```bash
   docker logs caddy -f 2>&1 | grep staging.api.solidrop
   ```

### 教訓

**Caddy への新ドメイン追加は DNS レコードが完全に伝播してから行うこと。** 理想的な順序:
1. DNS A レコード追加
2. `dig @1.1.1.1` で解決できることを確認 (TTL 300 秒なのでほぼ即時)
3. `caddy-apply.sh` 実行

---

## 検証結果

```bash
# HTTP → HTTPS リダイレクト確認
curl -v http://staging.api.solidrop.nafell.dev/
# → 308 Permanent Redirect

# HTTPS ヘルスチェック
curl https://staging.api.solidrop.nafell.dev/health
# → {"status":"ok"}
```

---

## 次のステップ

- prod 環境の同様セットアップ (`Caddyfile.prod.snippet`, `solidrop-api-prod.service`)
- CI/CD パイプラインによる自動デプロイ (`deploy.sh` の自動化)
- IAM アクセスキーのローテーション運用の確立
