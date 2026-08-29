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
| Source commit | `d6dbce9da8e636fb5e671e368dc7f464e0eb0a98` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-08-28 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

At vendoring time the `sw-tos` working tree had uncommitted changes to
`tools/te-rs/src/main.rs`. That file belongs to the `te-rs` terminal frontend and takes no
part in producing the image, so the artifacts still correspond to the commit
above. Recorded because a dirty source tree is otherwise invisible later.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 24,870 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,681,291 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:30c68b` |
| `build_id_size` | 8578 |
| `image_size` | 24870 |
| `image_sha256` | `1bc6834b9d2fd6b60fab8e2e5b18e64c886d355e611d70e6d6b10c49ff5d93b7` |

`build_id` is the identity the SWTOS debugger's identity opcode returns, as a
CRC over the image's immutable range. `crates/swtos-host/src/image.rs` mirrors
these values as constants and its tests assert that the embedded image still
matches the map, so a half-updated pair fails the build rather than producing
a demo that misreports its own symbols.

## Why the debug map is not compiled in

`program.bin` is embedded with `include_bytes!`. The debug map is **not**: at
1.6 MB it would dwarf a WASM bundle that is otherwise around 126 KB, and
`pages/` is committed, so every rebuild would add another copy of that bulk to
git history. Only the debugger pane needs the map, and it can fetch it as a
static asset when it is first opened. The map is embedded in the **test**
binary only, which is where the consistency check lives.
