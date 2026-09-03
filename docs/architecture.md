# web-sw-tos Architecture

This document records the deliberate module tree. It exists because
`sw-checklist` warns at more than 4 modules per crate and fails at more than 7,
warns at more than 4 functions per module and fails at more than 7, and warns
at functions over 25 lines and files over 350. A single-crate browser app of
this size cannot meet those limits, so the work is split across a small
workspace from the start rather than being refactored under pressure later.

## Workspace layout

```
web-sw-tos/                    root crate: the browser, and nothing else
  src/lib.rs                   App component; `mod` statements only
  src/main.rs                  Yew renderer entry point
  src/browser.rs               the only code that touches js_sys/web_sys
  src/chrome.rs                page chrome and the fitted geometry

crates/swtos-session/          pure session logic, no browser dependency
  src/state.rs                 declarations only: no impl blocks
  src/build.rs                 construction
  src/driver.rs                the run loop
  src/sending.rs               everything put on the wire
  src/routing.rs               inbound frames to panes
  src/debugger.rs              the Debugger pane's local console

crates/swtos-input/            keyboard, layered on the session
  src/translate.rs             key events to terminal bytes
  src/dispatch.rs              prefix, copy mode, console panes, target

crates/swtos-frontend/         vendored te-rs core
crates/swtos-host/             emulator, virtual UART, vendored image
```

Dependencies run one way: `web-sw-tos` -> `swtos-input` -> `swtos-session` ->
`swtos-frontend` and `swtos-host`. Nothing points back.

## Why this shape

`sw-checklist` is a forcing function for functional programming, loose
coupling, pure functions, delegation, and composition. Its limits are not
satisfied by compression, and never by an exception on code we author.

Two defects made that concrete. A `Mode::Plain` guard on the HELLO retry was
dropped while inlining a function to reduce a count, which let the browser
re-attach every 25 ticks and reprint the menu forever. A `CHANNEL_CLOSE` arm
was deleted to win back six lines. Both were **mechanical compression**, and
both lived in code no native test could reach, because `session.rs` and
`transport.rs` read `js_sys::Date` directly.

So the fix was design, not tidying:

- **Time is injected.** `state::Clock` is a trait; the browser implements it in
  `src/browser.rs` and tests supply a fake from `tests/`. That one change made
  the whole session testable, and the HELLO guard now has a test that fails if
  it is dropped again.
- **Data is separated from behaviour.** `state.rs` holds declarations with no
  `impl` blocks; the functions that act on them live in modules named for the
  job they do.
- **`Session` is composed**, not flat: `Transport`, `Panes`, `Input`, and
  `Console`, so routing never sees the UART and sending never sees the panes.
- **`lib.rs` holds only `mod` statements.**
- **Crates split by concern**, so module budgets fall out of the design rather
  than being fought.
- **Test doubles live in test code.** Nothing shipped exists for tests.

Remaining findings in these crates are warnings, never failures, and each is a
cohesive module at five to seven functions rather than a place to hide.

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

Current vendoring: **sw-tos f9197df**.

The host-command prefix is `Ctrl-O`. Passages below written when it was
`Ctrl-A` name it as it was, because they are describing when a decision was
made; the binding they describe is unchanged, only the key that reaches it.

| File | Kind | Change |
|---|---|---|
| `resource.rs` | required | `Instant` -> `Millis` (`f64`), caller-supplied. `Instant::now()` panics on wasm32. |
| `debug.rs` | required | `load(path)` -> `from_json(contents)`. No filesystem in a browser. |
| `ui.rs` | required | Adds `Cell`, `Color`, `Attrs`, and a `render_grid` **adapter** over upstream's `render`. |
| all five | trace | Provenance header naming source repo, path, commit, and date. |

A **tracking** patch is one that exists upstream but is not committed yet.
Vendoring from `git show HEAD:...` is right -- provenance must name a commit
that actually contains the content -- but it means uncommitted upstream work is
invisible here, and the two frontends then behave differently for no reason a
reader could see. Copy such a change verbatim, mark it tracking, and delete it
when the commit lands. Verify it is byte-identical, so the re-vendor produces
no diff at all.

`protocol.rs` is vendored unmodified and has now been unchanged upstream for
five cycles; it needs no patch at all. `disasm.rs` is new this cycle and is
also vendored unmodified -- it arrived with the ability to disassemble from
image bytes, so `dis` no longer depends on the debug map being loaded.

Only two kinds of patch are left, and both are forced by the platform: no
filesystem, and no `Instant`. Every patch that existed because this project
wanted different behaviour is gone.

### What this re-vendor taught

The prefix moved to `Ctrl-O`, and upstream's reason for the move is worth
keeping: `Ctrl-A` is beginning-of-line to anyone with emacs fingers and the
shell wants it back once its line editing grows up. `Ctrl-J` was tried first
and is a trap -- it is LF, so it arrives at the end of every pasted line and
would swallow the Enter.

The lesson that transfers is the second one in that commit. Upstream found its
two test harnesses spelling the prefix as a bare byte in a dozen places, with
one of them also passing `--prefix`, so the frontend under test and the keys
sent to it could name different keys. The tests here had the same shape, with
`"a"` written out at fourteen call sites. Both the arming and every label now
come from `dispatch::PREFIX_KEY` and `PREFIX_LABEL`, including the status line
and the help overlay, so the screen cannot advertise a key that does not work.

One assertion had to be pulled apart rather than renamed. A test read
`translate::to_bytes("a", true) == 0x01` under the comment "Ctrl-A is the
prefix", which quietly conflated two jobs: translating a control letter, and
intercepting the prefix before it reaches the target. They are separate now, so
the translation stays true whatever the prefix is.

### What the 4bfe19a re-vendor taught

`debug.rs` grew `help_lines()`, and taking it whole is the point: the debugger
pane's help is now upstream's list, so the two frontends cannot describe the
same commands differently. Only the display of it lives here.

Where the two do differ is when it is shown. Upstream reprints it on a rewind
by watching the shell's own banners rather than by remembering what it sent,
because a rewind can be asked for in ways the frontend never sees -- `kill 1`
or `reboot` typed at the prompt. That reasoning transfers exactly, so
`routing.rs` watches for the same two banners on channel zero. Watching what
we sent would have been simpler and would have missed the cases that matter.

### What the 5a25f22 re-vendor taught

Upstream ran rustfmt over the frontend this cycle, so all four vendored files
came back reformatted. That is churn the patch inventory absorbs without
comment, because the patches are described by what they do rather than by line
numbers, and re-vendoring copies fresh and reapplies them. The real content in
that diff was two lines: a help entry, and `iv`/`ir` for the interrupt
registers in the disassembler, where the shared ISA crate exposes `r6`/`r7`
but COR24 assembly accepts only the architectural aliases.

`Ctrl-A B` joins `Ctrl-A k` as an unframed escape, and the pairing is why
`control.rs` exists: a restart rewinds endpoint 1 and keeps everything, a
reboot tears the system down and builds it again. Both are two bytes read by
the interrupt handler.

The footer clock was the quiet find. It was only ever updated on the framed
path, so it stopped whenever the target did -- which is precisely when a clock
is worth reading. That was reported once as "the footer clock is not
incrementing" and diagnosed then as a target fault, which it also was; this
half of it survived. It now updates before the framed guard.

### What the 60e6a57 re-vendor taught

Nothing was deleted this time and nothing was added to the table: `ui.rs` grew
two-snapshot `(ended)` marking and a help line, and the adapter did not notice.
The work was all on this side of the boundary, which is the table doing its
job.

`Ctrl-A k` is the interesting one. Upstream sends the restart request raw on a
plain link and wrapped in a passthrough frame once the link is framed, because
`cor24-debug-adapter` sits in between reading frames and discards whatever is
not one. There is no adapter here -- the pump hands bytes straight to the
modeled UART -- so the mode distinction collapses and the request is always
raw. Porting the branch as written would have been faithful to the source and
wrong for this target. `swtos-host/src/control.rs` says so where a reader will
be standing when they wonder.

### What the e08fa4e re-vendor taught

The table lost a row for the second cycle running, and this time the deletions
came from outside it as well. Upstream grew `ended` on a pane,
`mark_live_endpoints`, `close_ended`, `name_channel`, `clear`, and
`clear_channel`, and stopped stealing focus on `add_application`. Each of those
had a local counterpart here: an ENDED title suffix, an endpoints-seen bitmask,
`clear_focused`, and an `add_without_focus`/`restore_focus` pair. All were
deleted rather than reconciled.

Two of them were actively in the way. `clear_focused` was the last additive
patch to a vendored file, and keeping it would have meant carrying a private
method beside an upstream one that did the same job. Worse, this project bound
`Ctrl-A l` and `Ctrl-A c` locally; upstream now binds the same two keys to
`clear` and `close_ended`, so the local interception was shadowing the real
commands. Deleting local code the moment upstream ships an equivalent is not
tidiness -- it is what stops the two frontends drifting into disagreement about
what a key means.

`panes.rs` is what is left: the part upstream genuinely does not do, which is
mapping this project's channels onto upstream's endpoints.

### What the previous re-vendor taught

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
they are not patches. Opening a pane on `ChannelOpen` and following the
resource snapshot live in `crates/swtos-session/`, this project's own frame
routing, and cost nothing at re-vendor time. There is no longer any exception:
with `clear_focused` gone, every vendored file is either unmodified or carries
only a platform-forced patch, which is why all vendored tests run unmodified --
including the two upstream added for short-lived processes.

## What is vendored, and from where

`../sw-tos` is read-only reference material and is never modified. The `te-rs`
modules and the prebuilt SWTOS image are **copies** living here, free to
diverge. Re-vendoring from a newer `sw-tos` is the routine way this repo tracks
upstream. See [plan.md](plan.md) for the per-module triage and the hazards.
