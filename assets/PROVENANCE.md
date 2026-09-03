# Vendored SWTOS artifacts

These files are **copies**, not build products. `sw-tos/build/` and its PL/SW
toolchain (`tools/bin/`) are both gitignored, so GitHub Actions can never
produce them. They are committed here so the browser demo can be built and
deployed from a clean checkout.

`sw-tos` itself is never modified by this project. Regenerate with
[`scripts/refresh-image.sh`](../scripts/refresh-image.sh).

## Source

| Field | Value |
|---|---|
| Source repository | `sw-embed/sw-tos` |
| Source commit | `7a6227e69a725a9fa33ba99a9807d6ba813fc5b2` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-09-01 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

This pair was built in a throwaway clone of `sw-tos` at `7a6227e`, with the
gitignored `tools/bin` toolchain copied in. `sw-tos` itself is never written
to. Building outside the source checkout is the only way to be certain which
tree an image came from, and it costs a clone; an earlier vendoring built this
way was later compared against the checkout's own build and was byte-identical,
SHA-256 and all.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 29,038 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,974,175 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:53999d` |
| `build_id_size` | 9730 |
| `image_size` | 29038 |
| `image_sha256` | `73f7582af361cb9495281ec17ab0440fa4b23f955872a469d922f7b78ef48f58` |

`build_id` is the identity the SWTOS debugger's identity opcode returns, as a
CRC over the image's immutable range. `crates/swtos-host/src/image.rs` mirrors
these values as constants and its tests assert that the embedded image still
matches the map, so a half-updated pair fails the build rather than producing
a demo that misreports its own symbols.

## Why the debug map is not compiled in

`program.bin` is embedded with `include_bytes!`. The debug map is **not**: at
1.7 MB it would dwarf a WASM bundle that is otherwise around 294 KB, and
`pages/` is committed, so every rebuild would add another copy of that bulk to
git history. Only the debugger pane needs the map, and it can fetch it as a
static asset when it is first opened. The map is embedded in the **test**
binary only, which is where the consistency check lives.
