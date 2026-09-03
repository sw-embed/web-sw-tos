#!/usr/bin/env bash
set -euo pipefail

# Cut a STABLE public release of the live demo.
#
# Two channels, deliberately different in how they move:
#
#   nightly  https://sw-embed.github.io/web-sw-tos/   moves on every push
#   stable   https://swtos.softwarewrighter.com/      moves only when you run this
#
# The difference is not just which files get copied. The rolling build bakes
# `--public-url /web-sw-tos/` into every asset URL for the project-pages
# subpath; a custom domain serves from the root, so the stable channel has to
# be REBUILT with `--public-url /`. Copying pages/ across would 404 on every
# asset. The runtime fetch of program.debug.json is relative and works either
# way, which is why it needs no rewriting here.
#
# First run creates ../sw-tos-live as a fresh git repo and commits; then:
#   1. create an EMPTY repo sw-embed/sw-tos-live on GitHub (no README/license)
#   2. git -C ../sw-tos-live remote add origin git@github.com:sw-embed/sw-tos-live.git
#   3. git -C ../sw-tos-live push -u origin main
#   4. Repo Settings -> Pages: Source = main / root; confirm the custom domain
#      (read from the CNAME file); enable Enforce HTTPS once the cert lands.
#   5. DNS: CNAME swtos -> sw-embed.github.io
# Later runs rebuild and commit; push to republish.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
REL_DIST="$PROJECT_DIR/dist-release"
MIRROR="${MIRROR:-$PROJECT_DIR/../sw-tos-live}"
DOMAIN="swtos.softwarewrighter.com"

# Refuse to ship what has not been through the gate. A stable release is the
# one build a stranger sees, so "I meant to run the tests" is not good enough.
if [ -n "$(git -C "$PROJECT_DIR" status --porcelain --untracked-files=no)" ]; then
    echo "error: working tree is dirty; commit or stash before cutting a release" >&2
    git -C "$PROJECT_DIR" status --short --untracked-files=no >&2
    exit 1
fi

echo "=== Gate ==="
"$SCRIPT_DIR/gate.sh"

echo "=== Building STABLE release (root base-path) ==="
cd "$PROJECT_DIR"
trunk build --release --public-url / --dist "$REL_DIST"

mkdir -p "$MIRROR"
# Mirror the built site. Preserve the repo's own .git and the files that are
# the repository's rather than the build's: the domain, the Jekyll opt-out,
# and its docs and legal text.
rsync -a --delete \
    --exclude='.git/' --exclude='CNAME' --exclude='.nojekyll' \
    --exclude='README.md' --exclude='LICENSE' --exclude='COPYRIGHT' \
    "$REL_DIST/" "$MIRROR/"

touch "$MIRROR/.nojekyll"
echo "$DOMAIN" >"$MIRROR/CNAME"

# Build provenance. The SWTOS image identity is recorded beside the source
# commit because it is the other half of what this demo is: two artifacts are
# vendored together and a release is only meaningful as the pair.
COMMIT=$(git -C "$PROJECT_DIR" rev-parse --short HEAD)
BUILT_AT=$(date -u +%Y-%m-%dT%H:%M:%SZ)
BUNDLE=$(ls "$MIRROR"/web-sw-tos-*_bg.wasm | head -1 | sed 's/.*\(web-sw-tos-[0-9a-f]*\)_bg.wasm/\1/')
IMAGE=$(python3 -c "import json;print(json.load(open('$PROJECT_DIR/assets/program.debug.json'))['build_id'])")
cat >"$MIRROR/build-info.json" <<INFO
{"commit":"$COMMIT","built_at":"$BUILT_AT","bundle":"$BUNDLE","image":"$IMAGE","channel":"stable"}
INFO
perl -pi -e "s|<head>|<head><meta name=\"swtos-build\" content=\"$COMMIT $BUILT_AT $IMAGE stable\">|" "$MIRROR/index.html"

if [ ! -d "$MIRROR/.git" ]; then
    git -C "$MIRROR" init -q -b main
    echo "Initialized $MIRROR as a fresh git repo (branch main)."
fi
git -C "$MIRROR" add -A
git -C "$MIRROR" commit -q -m "release: web-sw-tos $COMMIT (stable @ $BUILT_AT, image $IMAGE)" \
    || echo "(nothing new to commit)"

echo "=== Stable release built in $MIRROR (web-sw-tos $COMMIT, image $IMAGE) ==="
if ! git -C "$MIRROR" remote get-url origin >/dev/null 2>&1; then
    cat <<NEXT
Next (first time only):
  1. Create an EMPTY repo sw-embed/sw-tos-live on GitHub (no README/license).
  2. git -C $MIRROR remote add origin git@github.com:sw-embed/sw-tos-live.git
  3. git -C $MIRROR push -u origin main
  4. Repo Settings -> Pages: Source = main / root; custom domain $DOMAIN
     (from CNAME); enable Enforce HTTPS after the cert provisions.
  5. DNS: CNAME swtos -> sw-embed.github.io
NEXT
else
    echo "To publish: git -C $MIRROR push"
fi
