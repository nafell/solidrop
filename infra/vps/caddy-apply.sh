#!/usr/bin/env bash
# Apply Caddyfile snippets to the running Caddy container.
# Run this on the VPS after adding a new environment.
#
# Usage: bash infra/vps/caddy-apply.sh [staging|prod|all]
#
# What this does:
#   1. Locate the Caddyfile that is bind-mounted into the Caddy container
#   2. Append any missing snippet blocks (idempotent — skips if already present)
#   3. Reload Caddy in-place (zero-downtime)

set -euo pipefail

REPO_DIR="${REPO_DIR:-/opt/solidrop}"
CADDY_CONTAINER="${CADDY_CONTAINER:-caddy}"
TARGET="${1:-all}"

# --------------------------------------------------------------------------- #
# Helpers                                                                       #
# --------------------------------------------------------------------------- #

caddy_config_path() {
  # Resolve the host-side path of the Caddyfile mounted into the container.
  # Supports both file-level mount (/etc/caddy/Caddyfile) and
  # directory-level mount (/etc/caddy → host_dir, file is host_dir/Caddyfile).
  local file_mount dir_mount
  file_mount="$(docker inspect "${CADDY_CONTAINER}" \
    --format '{{range .Mounts}}{{if eq .Destination "/etc/caddy/Caddyfile"}}{{.Source}}{{end}}{{end}}')"
  if [[ -n "${file_mount}" ]]; then
    echo "${file_mount}"
    return
  fi
  dir_mount="$(docker inspect "${CADDY_CONTAINER}" \
    --format '{{range .Mounts}}{{if eq .Destination "/etc/caddy"}}{{.Source}}{{end}}{{end}}')"
  if [[ -n "${dir_mount}" ]]; then
    echo "${dir_mount}/Caddyfile"
    return
  fi
}

apply_snippet() {
  local snippet_file="$1"
  local marker="$2"   # unique string that must appear in the Caddyfile if already applied
  local host_caddyfile="$3"

  if grep -qF "${marker}" "${host_caddyfile}"; then
    echo "  [skip] '${marker}' already present in Caddyfile"
  else
    echo "  [add]  Appending ${snippet_file}"
    echo "" >> "${host_caddyfile}"
    cat "${snippet_file}" >> "${host_caddyfile}"
  fi
}

# --------------------------------------------------------------------------- #
# Main                                                                          #
# --------------------------------------------------------------------------- #

HOST_CADDYFILE="$(caddy_config_path)"
if [[ -z "${HOST_CADDYFILE}" ]]; then
  echo "ERROR: Could not find Caddyfile mount path in container '${CADDY_CONTAINER}'." >&2
  echo "       Check that the container is running and has /etc/caddy/Caddyfile mounted." >&2
  exit 1
fi
echo "==> Caddyfile path: ${HOST_CADDYFILE}"

case "${TARGET}" in
  staging|all)
    echo "==> Applying staging snippets..."
    apply_snippet \
      "${REPO_DIR}/infra/vps/Caddyfile.staging.snippet" \
      "staging.api.solidrop.nafell.dev" \
      "${HOST_CADDYFILE}"
    apply_snippet \
      "${REPO_DIR}/infra/vps/Caddyfile.web-staging.snippet" \
      "staging.web.solidrop.nafell.dev" \
      "${HOST_CADDYFILE}"
    ;;& # fall-through only if all
  prod|all)
    if [[ "${TARGET}" == "prod" || "${TARGET}" == "all" ]]; then
      PROD_SNIPPET="${REPO_DIR}/infra/vps/Caddyfile.prod.snippet"
      if [[ -f "${PROD_SNIPPET}" ]]; then
        echo "==> Applying prod API snippet..."
        apply_snippet \
          "${PROD_SNIPPET}" \
          "api.solidrop.nafell.dev" \
          "${HOST_CADDYFILE}"
      else
        echo "  [skip] ${PROD_SNIPPET} not found — skipping prod API"
      fi
      WEB_PROD_SNIPPET="${REPO_DIR}/infra/vps/Caddyfile.web-prod.snippet"
      if [[ -f "${WEB_PROD_SNIPPET}" ]]; then
        echo "==> Applying prod web snippet..."
        apply_snippet \
          "${WEB_PROD_SNIPPET}" \
          "web.solidrop.nafell.dev" \
          "${HOST_CADDYFILE}"
      else
        echo "  [skip] ${WEB_PROD_SNIPPET} not found — skipping prod web"
      fi
    fi
    ;;
  *)
    echo "Usage: $0 [staging|prod|all]" >&2
    exit 1
    ;;
esac

echo "==> Reloading Caddy..."
docker exec "${CADDY_CONTAINER}" caddy reload --config /etc/caddy/Caddyfile
echo "==> Done."
echo "    API  staging: curl https://staging.api.solidrop.nafell.dev/health"
echo "    Web  staging: curl https://staging.web.solidrop.nafell.dev/"
