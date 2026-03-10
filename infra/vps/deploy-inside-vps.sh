#!/usr/bin/env bash
# Run this script directly on the VPS (not from a local PC).
# Usage: bash infra/vps/deploy-inside-vps.sh [staging|prod]
#
# Prerequisites on the VPS:
#   - Git repository cloned at REPO_DIR (default: /opt/solidrop)
#   - systemd service file installed and enabled (solidrop-api-staging or solidrop-api-prod)
#   - /etc/solidrop/<env>.env configured
#   - Docker installed and running

set -euo pipefail

ENV="${1:-staging}"
REPO_DIR="${REPO_DIR:-/opt/solidrop}"

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

echo "==> [${ENV}] Pulling latest code..."
git -C "$REPO_DIR" pull --ff-only

echo "==> [${ENV}] Building Docker image: ${IMAGE_TAG}"
docker build \
  -f "$REPO_DIR/crates/api-server/Dockerfile" \
  -t "${IMAGE_TAG}" \
  "$REPO_DIR"

echo "==> [${ENV}] Restarting service: ${SERVICE_NAME}"
sudo systemctl restart "${SERVICE_NAME}"

echo "==> Waiting for service to start..."
sleep 3
sudo systemctl status "${SERVICE_NAME}" --no-pager

echo "==> Deploy complete: ${ENV}"
