#!/usr/bin/env bash
set -euo pipefail

# Build pages/ for GitHub Pages deployment.
# The Pages workflow deploys the COMMITTED pages/ directory, so run this and
# stage pages/ whenever web source changes.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

echo "=== Building pages/ ==="
cd "$PROJECT_DIR"
mkdir -p pages
touch pages/.nojekyll
trunk build --release --public-url /web-sw-tos/
rsync -a --delete --exclude='.nojekyll' dist/ pages/

echo "=== Done ==="
echo "Pages built in: $PROJECT_DIR/pages/"
echo "To deploy: git add pages/ && git commit && git push"
