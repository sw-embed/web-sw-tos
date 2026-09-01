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
| Source commit | `5a25f22f416b721f84dcd08a8cb0eaecb3169099` |
| Source path | `build/scheduled-shell/` |
| Vendored on | 2026-09-01 (re-vendored) |
| Build recipe | `just scheduled-shell-build` |

This pair was built in a throwaway clone of `sw-tos` at `5a25f22`, with the
gitignored `tools/bin` toolchain copied in, because the `sw-tos` checkout was
then on an earlier commit with hardware testing running against it and building
there would have replaced the artifacts under test. `sw-tos` itself was never
written to.

That clone has since been confirmed unnecessary and, more usefully, harmless:
once the checkout advanced to `5a25f22` and rebuilt, its `program.bin` was
byte-identical to the one vendored here, SHA-256 and all. The build is
reproducible across trees, which is what makes building outside the source
checkout a legitimate way to vendor rather than a shortcut.

## Artifacts

| File | Size | Role |
|---|---|---|
| `program.bin` | 28,265 bytes | The preemptive-multitasking image, loaded at address 0 |
| `program.debug.json` | 1,922,725 bytes | Symbol, function, and instruction map for the debugger pane |

## Identity

The debug map records the image it was generated for, and the two must always
be replaced together.

| Field | Value |
|---|---|
| `format` | `swtos-debug-v1` |
| `build_id` | `crc24:5224df` |
| `build_id_size` | 9488 |
| `image_size` | 28265 |
| `image_sha256` | `35df72c254ed1a15431287503f05ae4c8c625ac406fb71c00842f1db0aa18d0c` |

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
