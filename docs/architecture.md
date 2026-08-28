# web-sw-tos Architecture

This document records the deliberate module tree. It exists because
`sw-checklist` warns at more than 4 modules per crate and fails at more than 7,
warns at more than 4 functions per module and fails at more than 7, and warns
at functions over 25 lines and files over 350. A single-crate browser app of
this size cannot meet those limits, so the work is split across a small
workspace from the start rather than being refactored under pressure later.

## Workspace layout

```
web-sw-tos/                    root crate, the Yew application
  src/lib.rs                   App component, module declarations
  src/main.rs                  Yew renderer entry point
  src/footer.rs                build-info footer
  src/terminal.rs              fixed-size character grid  (step 006)
  src/keys.rs                  key capture and Ctrl-A prefix (step 007)

crates/swtos-frontend/         vendored te-rs core        (step 003)
  src/lib.rs
  src/protocol.rs              framed transport, unchanged from te-rs
  src/ui.rs                    Desktop pane model + render_grid()
  src/resource.rs              RESOURCE_SNAPSHOT assembler
  src/debug.rs                 debugger console + debug map

crates/swtos-host/             emulator side
  src/lib.rs
  src/image.rs                 vendored SWTOS image + identity   (done)
  src/uart.rs                  in-process virtual UART
  src/pump.rs                  run_batch driver + 100 Hz heartbeat

assets/                        vendored, not built
  program.bin                  embedded with include_bytes!
  program.debug.json           served as a static asset, not compiled in
  PROVENANCE.md                source commit, sizes, and identity table
```

Each crate stays at or under seven modules, and two of the three stay at or
under five. The split also mirrors the CLI's own process boundary: `swtos-host`
is what `tools/cor24-debug-adapter` is, `swtos-frontend` is what `tools/te-rs`
is, and `swtos-host`'s `uart.rs` is what the pty is.

## Build order

Crates are added to `[workspace] members` as they gain content. Declaring a
member directory that does not exist is a hard cargo error, so the root crate
ships alone in step 001 and the other two join at steps 002/003.

The root crate carries the `cor24-emulator` and `cor24-isa` path dependencies
in step 001 so the scaffold proves the crate is wasm-clean. Those dependencies
move to `swtos-host` once that crate exists.

## Dependency direction

```
web-sw-tos  ->  swtos-frontend
            ->  swtos-host  ->  cor24-emulator  ->  cor24-isa
```

`swtos-frontend` and `swtos-host` never depend on each other. They are joined
only in the root crate, which owns the event loop: bytes out of the pump go
into the frontend decoder, and bytes out of the frontend go into the pump.
That is the same shape as the CLI, where the two processes share nothing but
the byte stream.

## Vendored code and sw-checklist

`sw-checklist` limits apply to code this project owns. They are relaxed for
files under `crates/swtos-frontend/src/` that are faithful copies of `te-rs`.

`protocol.rs` fails Module Function Count with 25 functions against a maximum
of 7, and warns on file length and two `push` implementations. Those limits
could be satisfied by splitting the module into `frame` / `decoder` /
`negotiation` submodules, and that is deliberately not done: the value of
vendoring is that re-syncing with upstream is a copy plus a header, and a
structural refactor turns every future re-vendor into a manual merge. Moving
the tests out does not help either -- it clears the file-length warning but
still leaves 18 functions.

So the trade is a permanent, well-understood exception on a small number of
copied files, versus permanent merge friction on every update to the
reference implementation. The exception is the cheaper side.

The rule this sets: **a vendored file carries an exception; a file we author
does not.** If a vendored module is ever rewritten rather than copied, it
loses the exception and must meet the limits like anything else. Commits that
add or refresh vendored code carry a `sw-checklist: exception` trailer naming
the file and the reason.

## What is vendored, and from where

`../sw-tos` is read-only reference material and is never modified. The `te-rs`
modules and the prebuilt SWTOS image are **copies** living here, free to
diverge. Re-vendoring from a newer `sw-tos` is the routine way this repo tracks
upstream. See [plan.md](plan.md) for the per-module triage and the hazards.
