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
| Source commit | `e08fa4ed9edce356bdc7b6d20df18928f5bee34b` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-08-31 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

`sw-tos` has since moved to `5839aad`, which is **not** vendored here. Those
two commits change only the image source -- `ps` reports `WAITING` rather than
`BLOCKED`, and `ps -l` became the same report `mon` prints, from the same code
-- and no built artifacts exist for them: `build/scheduled-shell/` still holds
`crc24:500933`, and the working tree is mid-edit. Building there to get ahead
would mean writing into a repository this project treats as read-only, and
would produce an image matching no commit at all. The vendored pair is
therefore the newest one that is both built and identifiable.

Nothing here reads those state names, so the gap costs only the two new
behaviours, not correctness.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 26,948 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,824,948 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:500933` |
| `build_id_size` | 8997 |
| `image_size` | 26948 |
| `image_sha256` | `e09f316a5a93fd4c6474aee2d8e6772488ba215610abb254921cff0f14f38592` |

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
