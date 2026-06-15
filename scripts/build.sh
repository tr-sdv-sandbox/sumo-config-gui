#!/usr/bin/env bash
# Build the SUMO Config GUI desktop bundle from any working directory.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

cd "$APP_DIR"

if [[ ! -d node_modules ]]; then
	npm install
fi

exec npm run tauri -- build "$@"
