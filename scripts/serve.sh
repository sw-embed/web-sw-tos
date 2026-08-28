#!/usr/bin/env bash
set -euo pipefail

# Development server for the SWTOS live demo.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$(dirname "$SCRIPT_DIR")"

exec trunk serve --port "${PORT:-9958}" "$@"
