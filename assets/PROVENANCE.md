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
| Source commit | `9a211905d704ca87328f3b5b8b7a1f415c7c9bbf` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-09-01 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

This pair was built in a throwaway clone of `sw-tos` at `9a21190`, with the
gitignored `tools/bin` toolchain copied in. `sw-tos` itself is never written
to. Building outside the source checkout is the only way to be certain which
tree an image came from, and it costs a clone.

The image is 951 bytes smaller than the last one and carries 537 fewer
instructions, from a shell refactor that prints strings by name through macros
rather than a character at a time. Nothing it does changed: every test here
reads the rendered screen, and all of them passed against the new image without
an edit.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 28,087 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,895,128 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:0158b8` |
| `build_id_size` | 9730 |
| `image_size` | 28087 |
| `image_sha256` | `447ba018a47dc286a221ccb3275151b54d7c479c01c468e4844dc7251a595a7f` |

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
