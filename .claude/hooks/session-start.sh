#!/bin/bash
set -euo pipefail

# Only run in remote (Claude Code on the web) environments
if [ "${CLAUDE_CODE_REMOTE:-}" != "true" ]; then
  exit 0
fi

cd "$CLAUDE_PROJECT_DIR"

# Ensure rustup components are available
rustup component add clippy rustfmt 2>/dev/null || true

# Build workspace to fetch and compile all dependencies (cached across sessions)
cargo build --all-targets 2>&1

# Verify clippy and fmt are functional
cargo clippy --version
cargo fmt --version
