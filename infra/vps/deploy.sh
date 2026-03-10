#!/usr/bin/env bash
# Usage: VPS_HOST=user@<IP> bash infra/vps/deploy.sh [staging|prod] [api|web|all]
#
# Examples:
#   VPS_HOST=user@<IP> bash infra/vps/deploy.sh staging api
#   VPS_HOST=user@<IP> bash infra/vps/deploy.sh prod web
#   VPS_HOST=user@<IP> bash infra/vps/deploy.sh staging all
#
# Must be run from repository root (where Cargo.toml is).

set -euo pipefail

ENV="${1:-staging}"
TARGET="${2:-all}"
VPS_HOST="${VPS_HOST:?VPS_HOST is required. Example: VPS_HOST=user@1.2.3.4 bash infra/vps/deploy.sh staging}"

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
  docker build -f crates/api-server/Dockerfile -t "${image_tag}" .

  echo "==> [${ENV}] Transferring API image to VPS (${VPS_HOST})..."
  docker save "${image_tag}" | ssh "${VPS_HOST}" "docker load"

  echo "==> [${ENV}] Restarting API service: ${service_name}"
  ssh "${VPS_HOST}" "sudo systemctl restart ${service_name}"
  sleep 3
  ssh "${VPS_HOST}" "sudo systemctl status ${service_name} --no-pager"
}

deploy_web() {
  local image_tag="solidrop-web:${ENV}"
  local service_name="solidrop-web-${ENV}"

  echo "==> [${ENV}] Building Web image: ${image_tag}"
  docker build -f web/Dockerfile -t "${image_tag}" web/

  echo "==> [${ENV}] Transferring Web image to VPS (${VPS_HOST})..."
  docker save "${image_tag}" | ssh "${VPS_HOST}" "docker load"

  echo "==> [${ENV}] Restarting Web service: ${service_name}"
  ssh "${VPS_HOST}" "sudo systemctl restart ${service_name}"
  sleep 3
  ssh "${VPS_HOST}" "sudo systemctl status ${service_name} --no-pager"
}

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
