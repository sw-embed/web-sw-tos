#!/usr/bin/env bash
set -euo pipefail

# The full pre-commit gate, as one command that exits non-zero on any failure.
# Written after two occasions where a chain of `&& echo PASS` swallowed a
# failing step: a gate that cannot report red is worse than no gate.

cd "$(dirname "$0")/.."
LOCAL="-p web-sw-tos -p swtos-frontend -p swtos-host -p swtos-session -p swtos-input"

echo "== fmt"
# shellcheck disable=SC2086
cargo fmt $LOCAL -- --check

echo "== clippy"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "== tests"
cargo test --workspace

echo "== wasm"
cargo build --workspace --target wasm32-unknown-unknown

echo "== sw-checklist"
# `sw-checklist` exits non-zero while the vendored crates carry their
# documented structural exception, so its status cannot be the gate. What must
# hold is that code we author has no failures at all.
REPORT=$(sw-checklist -v || true)
MINE=$(grep -E "web-sw-tos\]|swtos-host\]|swtos-session\]|swtos-input\]" <<<"$REPORT" | grep "FAIL" || true)
if [ -n "$MINE" ]; then
    echo "sw-checklist: failures in self-authored crates" >&2
    echo "$MINE" >&2
    exit 1
fi
tail -1 <<<"$REPORT"

echo "== gate passed"
