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

## Vendored code: what is excepted, and what is not

The two quality gates are treated differently, because they fail for different
reasons.

**Clippy is fixed, never suppressed -- vendored files included.** Every clippy
finding in vendored code so far has been a small, local, mechanical fix that
survives re-vendoring as a short patch: collapsing a nested `if`, bundling four
positional arguments into a `Rect`, iterating a row instead of indexing it by
range. None required understanding the module as a whole. There is no
`#[allow]` anywhere in this repo, and there should not be one.

**`sw-checklist`'s structural limits are excepted for vendored files.** These
are the limits on functions per module, lines per function, and lines per file.
Satisfying them means splitting modules and reshaping functions, which forks
the file from upstream and turns every future re-vendor into a manual merge
rather than a copy plus a header. That is a permanent cost paid on every
update, against a bounded and well-understood one-time exception.

Current standing exception, all of it `crates/swtos-frontend/src/`:

| Module | Why it fails |
|---|---|
| `protocol.rs` | 25 functions, 445 lines |
| `ui.rs` | 50 functions, 730 lines |
| `debug.rs` | 31 functions, 567 lines |
| `resource.rs` | 8 functions, one 69-line `push` |

The crate itself also warns at 5 modules against a limit of 4.

Two self-authored warnings are accepted, both structural and both the tree
this document planned from the start: the root crate holds 5 modules
(`lib`, `main`, `chrome`, `session`, `keys`) and `session.rs` holds 5
functions, against a warning threshold of 4 and a failure threshold of 7.
Forcing either down means merging things that do not belong together --
key translation back into the session, or page chrome into the app root.
Every *function-level* warning has been fixed as it appeared.

The rule this sets: **a vendored file carries the structural exception; a file
we author does not**, except where the planned module tree exceeds the warning
threshold and the alternative is worse code. Warnings introduced in `src/` or `crates/swtos-host/`
have been fixed each time they appeared rather than allowed to accumulate. If a
vendored module is ever rewritten rather than copied, it loses the exception.
Commits that add or refresh vendored code carry a `sw-checklist: exception`
trailer naming the files and the reason.

## Vendored patch inventory

Every local change to a vendored file, so re-vendoring is mechanical rather
than archaeological. Re-vendor by copying upstream fresh, then walking this
table.

Current vendoring: **sw-tos d6dbce9**.

| File | Kind | Change |
|---|---|---|
| `resource.rs` | required | `Instant` -> `Millis` (`f64`), caller-supplied. `Instant::now()` panics on wasm32. |
| `debug.rs` | required | `load(path)` -> `from_json(contents)`. No filesystem in a browser. |
| `ui.rs` | required | Adds `Cell`, `Color`, `Attrs`, and a `render_grid` **adapter** over upstream's `render`. |
| `ui.rs` | additive | Adds `clear_focused`. Upstream has no clear command at all, so its only way to discard a pane's output is to close the pane. |
| `ui.rs` | tracking | Form-feed (`0x0c`) clears a pane. Copied byte-identical from upstream's **uncommitted** working tree; delete on the re-vendor that brings it in. |
| all three | trace | Provenance header naming source repo, path, commit, and date. |

A **tracking** patch is one that exists upstream but is not committed yet.
Vendoring from `git show HEAD:...` is right -- provenance must name a commit
that actually contains the content -- but it means uncommitted upstream work is
invisible here, and the two frontends then behave differently for no reason a
reader could see. Copy such a change verbatim, mark it tracking, and delete it
when the commit lands. Verify it is byte-identical, so the re-vendor produces
no diff at all.

`protocol.rs` is vendored unmodified and was unchanged upstream across this
cycle; it needs no patch at all.

### What the last re-vendor taught

The three clippy patches this table used to carry are **gone**, exactly as
planned: upstream cleared its own clippy findings in `71ff551`, including
`too_many_arguments` on `draw_box`, which it solved with a `BoxSpec` struct
rather than the `Rect` used here. Upstream's shape won, the local patches were
deleted rather than reconciled, and nothing was kept alive merely because it
existed.

The `ui.rs` patch changed character, and this is the durable lesson. It began
as an invasive conversion of every canvas write from `char` to `Cell`. Upstream
then grew `ui.rs` by 375 lines in one cycle -- new separators, borders removed,
sixteen slots -- and that patch would have had to be redone by hand against
substantially rewritten code. It is now a thin adapter: `render_grid` calls
upstream's `render` and strips the three escapes it emits (`\x1b[H`, a
per-line `\x1b[K`, `\r\n`). The adapter survived the rewrite untouched, and
picked up the new pane chrome for free.

The rule that follows: **adapt at the boundary, do not rewrite the interior.**
A patch that touches one function survives upstream churn; a patch spread
across a module does not. The same reasoning is why the vendored tests now run
completely unmodified -- upstream's `render` is left intact, so its own
assertions still exercise it.

When colour arrives it enters as SGR in pane content, and `render_grid` is the
one place that has to learn to parse it.

The pane-lifecycle divergences are deliberately **not** in this table, because
they are not patches. Ended panes, clearing on channel reuse, and opening a
pane on `ChannelOpen` all live in `src/transport.rs`, this project's own frame
routing, and cost nothing at re-vendor time. `clear_focused` is the single
exception: emptying a pane needs the private field, so it is one additive
method rather than a change to existing logic.

## What is vendored, and from where

`../sw-tos` is read-only reference material and is never modified. The `te-rs`
modules and the prebuilt SWTOS image are **copies** living here, free to
diverge. Re-vendoring from a newer `sw-tos` is the routine way this repo tracks
upstream. See [plan.md](plan.md) for the per-module triage and the hazards.
