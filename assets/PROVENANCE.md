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
| Source commit | `4bfe19a40a634d8d2243cf10ede1c4b233b6047c` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-09-01 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

This pair was built in a throwaway clone of `sw-tos` at `4bfe19a`, with the
gitignored `tools/bin` toolchain copied in. `sw-tos` itself is never written
to. Building outside the source checkout was first done because hardware
testing was running against it; it is kept because it is the only way to be
certain which tree an image came from, and it costs a clone.

It is verifiably the same image either way. The previous vendoring was built
this way and then compared against the `sw-tos` checkout once that caught up:
byte-identical, SHA-256 and all. This one matches the `build_id` of the
checkout's own build too.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 28,308 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,925,294 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:d10c7a` |
| `build_id_size` | 9488 |
| `image_size` | 28308 |
| `image_sha256` | `acc21b3f6dc57843b07b3a4682a9792b4143a680c43f3f039fe9dedc9e55105f` |

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
