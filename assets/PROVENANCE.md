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
| Source commit | `60e6a5748a348b42b2be39906cbfd2c84964e002` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-09-01 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

The `sw-tos` working tree was clean at this commit and the built artifacts
postdate every image source it contains, so the pair below corresponds to
`60e6a57` and not to work in progress. The previous vendoring deliberately
stopped two commits short for exactly that reason; this one does not have to.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 27,883 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,894,590 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:472414` |
| `build_id_size` | 9356 |
| `image_size` | 27883 |
| `image_sha256` | `104cc79add85df211daafb319402e40dab6121626511c1e4532c9c79556046d2` |

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
