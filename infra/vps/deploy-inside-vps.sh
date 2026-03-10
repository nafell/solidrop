#!/usr/bin/env bash
# Run this script directly on the VPS (not from a local PC).
# Usage: bash infra/vps/deploy-inside-vps.sh [staging|prod] [api|web|all]
#
# Examples:
#   bash infra/vps/deploy-inside-vps.sh staging api
#   bash infra/vps/deploy-inside-vps.sh prod web
#   bash infra/vps/deploy-inside-vps.sh staging all
#
# Prerequisites on the VPS:
#   - Git repository cloned at REPO_DIR (default: /opt/solidrop)
#   - systemd service files installed and enabled
#   - /etc/solidrop/<env>.env configured (API only)
#   - Docker installed and running

set -euo pipefail

ENV="${1:-staging}"
TARGET="${2:-all}"
REPO_DIR="${REPO_DIR:-/opt/solidrop}"

case "$ENV" in
  staging|prod) ;;
  *)
    echo "Unknown environment: $ENV. Use 'staging' or 'prod'." >&2
    exit 1
    ;;
esac

deploy_api() {
  local image_tag="solidrop-api-server:${ENV}"
  local service_name="solidrop-api-${ENV}"

  echo "==> [${ENV}] Building API image: ${image_tag}"
  docker build \
    -f "$REPO_DIR/crates/api-server/Dockerfile" \
    -t "${image_tag}" \
    "$REPO_DIR"

  echo "==> [${ENV}] Restarting API service: ${service_name}"
  sudo systemctl restart "${service_name}"
  sleep 3
  sudo systemctl status "${service_name}" --no-pager
}

deploy_web() {
  local image_tag="solidrop-web:${ENV}"
  local service_name="solidrop-web-${ENV}"

  echo "==> [${ENV}] Building Web image: ${image_tag}"
  docker build \
    -f "$REPO_DIR/web/Dockerfile" \
    -t "${image_tag}" \
    "$REPO_DIR/web"

  echo "==> [${ENV}] Restarting Web service: ${service_name}"
  sudo systemctl restart "${service_name}"
  sleep 3
  sudo systemctl status "${service_name}" --no-pager
}

echo "==> [${ENV}] Pulling latest code..."
git -C "$REPO_DIR" pull --ff-only

case "$TARGET" in
  api)  deploy_api ;;
  web)  deploy_web ;;
  all)  deploy_api; deploy_web ;;
  *)
    echo "Unknown target: $TARGET. Use 'api', 'web', or 'all'." >&2
    exit 1
    ;;
esac

echo "==> Deploy complete: ${ENV}/${TARGET}"
