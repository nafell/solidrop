#!/usr/bin/env bash
# Usage: VPS_HOST=user@<IP> bash infra/vps/deploy.sh staging
#        VPS_HOST=user@<IP> bash infra/vps/deploy.sh prod
#
# Must be run from repository root (where Cargo.toml is).

set -euo pipefail

ENV="${1:-staging}"
VPS_HOST="${VPS_HOST:?VPS_HOST is required. Example: VPS_HOST=user@1.2.3.4 bash infra/vps/deploy.sh staging}"

case "$ENV" in
  staging)
    IMAGE_TAG="solidrop-api-server:staging"
    SERVICE_NAME="solidrop-api-staging"
    ;;
  prod)
    IMAGE_TAG="solidrop-api-server:prod"
    SERVICE_NAME="solidrop-api-prod"
    ;;
  *)
    echo "Unknown environment: $ENV. Use 'staging' or 'prod'." >&2
    exit 1
    ;;
esac

echo "==> [${ENV}] Building Docker image: ${IMAGE_TAG}"
docker build -f crates/api-server/Dockerfile -t "${IMAGE_TAG}" .

echo "==> [${ENV}] Transferring image to VPS (${VPS_HOST})..."
docker save "${IMAGE_TAG}" | ssh "${VPS_HOST}" "docker load"

echo "==> [${ENV}] Restarting service: ${SERVICE_NAME}"
ssh "${VPS_HOST}" "sudo systemctl restart ${SERVICE_NAME}"

echo "==> Waiting for service to start..."
sleep 3
ssh "${VPS_HOST}" "sudo systemctl status ${SERVICE_NAME} --no-pager"

echo "==> Deploy complete: ${ENV}"
