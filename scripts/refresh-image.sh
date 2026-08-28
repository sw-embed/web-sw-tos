#!/usr/bin/env bash
set -euo pipefail

# Re-vendor the SWTOS image and debug map from a sibling sw-tos checkout, and
# rewrite assets/PROVENANCE.md to match.
#
# sw-tos is READ ONLY to this project: this script copies out of it and never
# writes to it. Build the artifacts there first with `just scheduled-shell-build`.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"
SWTOS="${SWTOS:-$(dirname "$PROJECT_DIR")/sw-tos}"
SRC="$SWTOS/build/scheduled-shell"

[ -d "$SWTOS" ] || { echo "error: no sw-tos checkout at $SWTOS" >&2; exit 1; }
for f in program.bin program.debug.json; do
    [ -f "$SRC/$f" ] || {
        echo "error: missing $SRC/$f" >&2
        echo "       build it in sw-tos first: just scheduled-shell-build" >&2
        exit 1
    }
done

SRC_SHA="$(cd "$SWTOS" && git rev-parse HEAD)"
DIRTY="$(cd "$SWTOS" && git status --porcelain | awk '{print $2}' | paste -sd' ' -)"
[ -n "$DIRTY" ] && echo "warning: sw-tos working tree is dirty: $DIRTY" >&2

# Refuse a mismatched pair before it reaches the repository. The map records
# the image it was generated for; shipping a half-updated pair would produce a
# demo that misreports its own symbols.
python3 - "$SRC" <<'PY'
import hashlib, json, sys, pathlib
src = pathlib.Path(sys.argv[1])
image = (src / "program.bin").read_bytes()
dmap = json.loads((src / "program.debug.json").read_text())
assert dmap["format"] == "swtos-debug-v1", f"unexpected format {dmap['format']}"
assert dmap["image_size"] == len(image), "image_size disagrees with program.bin"
digest = hashlib.sha256(image).hexdigest()
assert dmap["image_sha256"] == digest, "image_sha256 disagrees with program.bin"
print(f"pair verified: {dmap['build_id']} ({len(image)} bytes)")
PY

cp "$SRC/program.bin" "$PROJECT_DIR/assets/program.bin"
cp "$SRC/program.debug.json" "$PROJECT_DIR/assets/program.debug.json"

echo
echo "Copied from $SRC (sw-tos $SRC_SHA)"
echo
echo "Now update by hand, then run the tests:"
echo "  - assets/PROVENANCE.md      source commit, sizes, and identity table"
echo "  - crates/swtos-host/src/image.rs  BUILD_ID, IMAGE_SIZE, IMAGE_SHA256"
echo "  - cargo test -p swtos-host  proves the pair and the constants agree"
