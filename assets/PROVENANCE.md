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
| Source commit | `1e75960e1b0751bdf42c103412a9b18eafc25329` |
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
| `program.bin` | 25,529 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,728,900 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:67d633` |
| `build_id_size` | 8684 |
| `image_size` | 25529 |
| `image_sha256` | `1f1480bf12e0657566565dee89e11d0fc0fb8dec1bf551b0471dbfea7f3943f6` |

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
