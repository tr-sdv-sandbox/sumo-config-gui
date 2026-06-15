#!/usr/bin/env bash
# Run the SUMO Config GUI desktop app in development mode from any working directory.
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
APP_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

cd "$APP_DIR"

# Tower 2 is the default source. Set SUMO_CONFIG_GUI_ROOT only for local
# file-mode smoke tests, e.g. SUMO_CONFIG_GUI_ROOT=$APP_DIR/examples ./scripts/run.sh

if [[ ! -d node_modules ]]; then
	npm install
fi

exec npm run tauri -- dev "$@"
