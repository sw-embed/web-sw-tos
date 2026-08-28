# web-sw-tos -- Browser Live Demo Plan

Browser-hosted live demo of SWTOS: the COR24 emulator running the
preemptive-multitasking image, driven by a vendored copy of the `te-rs`
tiled terminal frontend, joined by an in-process virtual UART instead of a
pty, and rendered as a fixed-size character grid in a web page.

Live demo target: <https://sw-embed.github.io/web-sw-tos/>

## What this replaces

The CLI demo (`sw-tos`, `just cor24-debugger-demo`) is three pieces joined by
a pty, created by `scripts/swtos-emulator-debug.py`:

| Piece | Role |
|---|---|
| `tools/cor24-debug-adapter` | Hosts `EmulatorCore`, loads `program.bin` at 0, `run_batch(50_000)` in a 1 ms loop, decodes inbound frames, flushes modeled UART bytes out |
| pty pair | Byte transport between the two processes |
| `tools/te-rs` | Framed-transport decoder, tiled pane `Desktop`, Ctrl-A prefix commands, copy-mode scrollback, resource and debugger consoles |

In the browser all three collapse into one WASM module. The pty becomes two
in-process byte queues. Everything else keeps its shape.

## Decisions

- **Image**: vendor prebuilt `program.bin` and `program.debug.json` as tracked
  assets. `sw-tos/build/` and its PL/SW toolchain (`tools/bin/`) are both
  gitignored, so GitHub Actions can never build the image. Provenance (source
  `sw-tos` commit and the debug map's `crc24:` build id) is recorded alongside.
  `program.bin` is embedded with `include_bytes!`; the **debug map is not**.
  At 1.6 MB it would dwarf a bundle that is otherwise around 126 KB, and
  because `pages/` is committed, every rebuild would add another copy of that
  bulk to git history. Only the debugger pane needs it, and it can fetch the
  map as a static asset when first opened. The map is embedded in the test
  binary only, where the image/map consistency check lives.
- **Frontend core**: vendor a copy of the `te-rs` modules into this repo.
  `sw-tos` does not grow a WASM feature and does not depend on this repo.
- **Rendering**: one fixed-size character grid (selectable 80x24 / 120x43),
  painted from the pane canvas. No mouse, no scrollbars, no copy/paste. Pane
  layout, focus, zoom, and scrolling are driven entirely by Ctrl-A commands,
  exactly as in the CLI.
- **Delivery**: shell-first. Phase 1 ships one interactive Shell pane and is
  independently deployable; later phases add Application, Resources, Debugger,
  zoom, and copy mode.

## Vendored module triage

| Module | Verdict |
|---|---|
| `protocol.rs` | Pure. Drops in unchanged. |
| `ui.rs` | `VecDeque` + serde. Needs a `render_grid(w, h) -> Vec<Vec<Cell>>` that skips the three ANSI escapes `render()` emits. Cells rather than `char` from day one; see the ANSI-color section. |
| `resource.rs` | Uses `std::time::Instant`, which panics on `wasm32-unknown-unknown`. Needs a time source abstraction. |
| `debug.rs` | Loads the debug map with `std::fs`. Needs a `from_str` constructor. |
| `main.rs` | termios and raw-fd bound. Not vendored; its event loop is rewritten as the app. |

## Preemption

Preemption is host-driven and must be preserved. `te-rs` writes a five-byte
`FF 01 tick24` scheduler heartbeat every 10 ms. On the emulator path the
heartbeat is wrapped in a `RAW_UART` (`0xfe`) frame and injected into the
modeled UART at `HEARTBEAT_BYTE_CYCLES = 20_000`, well below the
`TARGET_BYTE_CYCLES = 500_000` used for ordinary frames. The browser pump owns
the same 100 Hz cadence.

The acceptance proof is unchanged from the CLI: run two `cpu-hog` processes
(neither contains a yield, syscall, I/O, IPC, sleep, or blocking operation),
and show the Resources pane's `fp=` forced-preemption counts and `cpu=` samples
advancing while Shell, Resources, and Debugger all stay responsive.

## Measured throughput

Taken at step 008, the step this plan was reordered to reach early. The
question was whether WASM sustains the 100 Hz scheduler heartbeat.

| Where | ms per tick | Effective rate |
|---|---|---|
| Native, release | 2.46 | 400 Hz equivalent |
| WASM, `opt-level = 3` | 18.73 | 54 Hz |
| WASM, `opt-level = "z"` | 16.78 | 61 Hz |

A tick is 5 heartbeat bytes at `HEARTBEAT_BYTE_CYCLES` plus one `BATCH`, so
150,000 emulated cycles. WASM runs about 7x slower than native here.

`opt-level = "z"` is both faster and smaller than `opt-level = 3` (176 KB
against 220 KB). That looks backwards until you remember the hot loop is an
instruction decoder: the smaller build fits the dispatch path in cache, and
the size win is free.

Both WASM figures come from a **hidden** browser tab, which Chrome
deprioritizes, so treat them as a pessimistic floor rather than what a viewer
sees. Two measurement traps are worth recording, because both produced
confidently wrong numbers first:

- A hidden tab throttles timers to about 1 Hz. Measuring ticks per second
  through a `setInterval` measured the throttle, not the emulator, and
  reported 961 ms per tick.
- Because of that throttle, work per callback must be derived from the wall
  clock rather than assumed to be one tick. This is why the app catches up.

### What ~60 Hz costs

The heartbeat carries SWTOS's sense of time, so at 61 Hz its clock runs at
about 60% of wall clock. Concretely:

- **Shell responsiveness is unaffected.** Input latency is bounded by one
  tick, around 17 ms, which is imperceptible.
- **Preemption still works.** Forced preemption is driven by heartbeat
  *count*, not by wall-clock rate, so the `cpu-hog` acceptance proof holds
  and `fp=` counters still advance.
- **Uptime and Clock run slow**, by the same 40%. This is the one visible
  artifact.

If the clock needs to be right, the lever is cycles per tick rather than the
tick rate. `BATCH` is 50,000 of the 150,000 cycles and exists only to let the
CPU run on beyond heartbeat pacing; shrinking it trades emulated CPU speed for
a more accurate clock. Not tuned yet -- 60 Hz is good enough to build on, and
tuning against a hidden-tab measurement would be tuning against noise.

## Pane lifecycle, and a deliberate divergence from te-rs

How `te-rs` behaves today, read from `ui.rs` and the frame dispatch in
`main.rs`:

| Frame | Effect |
|---|---|
| `ChannelOpen` (3) | `add_application`. If a pane already holds that channel it is **reused and retitled**, but its scrollback is **not** cleared. |
| `ChannelTitle` (5) | Retitles the pane at any time, whenever the target sends it. |
| `ChannelClose` (4) | `release_channel` **removes the application pane outright**. Shell, Debugger, and Resources panes are retained regardless. |

So today: a pane cannot be cleared at all -- there is no clear command in the
Ctrl-A set, and the only way to discard output is `Ctrl-A x` to close the pane
entirely. Titles change on open and on any `ChannelTitle` frame. Nothing is
ever flagged as ended, because an ended application's pane has already
disappeared.

Two of those are worth changing here, and this repo is free to: the vendored
copy may diverge, provided the divergence is deliberate and written down.

**An ended application keeps its pane, flagged.** Removing the pane on
`ChannelClose` destroys the program's final output at the exact moment the
output becomes worth reading. The pane stays, its title gains an ` (ended)`
suffix, and it stops accepting input. `Ctrl-A x` still closes it. This also
answers the stale-output question directly: output after an app ends is not
stale, it is the result, and it should persist until dismissed.

**Reuse clears; an explicit clear exists.** When a channel is reused for a new
application the pane is cleared first, so one program's output can never be
mistaken for the next one's. Separately, `Ctrl-A c` clears the focused pane on
demand -- scrollback, the partial line, and the scroll offset.

Both belong to the application-panes step. Recording them here because the
choice is not obvious from the reference implementation, and a later reader
comparing the two frontends should find the reason rather than assume drift.

## Known hazards

- `UartLog` is an unbounded `Vec` with only `clear()`. The CLI adapter's
  `uart_log_seen` cursor never resets, which is harmless in a short-lived
  process and a leak in a long browser session. The browser pump drains and
  resets instead.
- `EmulatorCore::get_uart_output()` returns `&str`. The framed transport is
  binary, so the byte-exact path is `uart_log().entries()` filtered by
  `UartDirection::Output`, never the string accessor.
- Ctrl-A is select-all in a browser. The key handler must intercept the prefix
  and `preventDefault`.
- The emulator crate reaches `std::fs` in its SPI peripherals and
  `SystemTime::now()` in the I2C registry. SWTOS uses UART only here, but the
  wasm32 build must be verified early rather than assumed.

## Phases

**Phase 0 -- foundation**

1. `scaffold-yew-project` -- Cargo.toml (edition 2024), Trunk `index.html`,
   `scripts/serve.sh`, `scripts/build-pages.sh`, Pages workflow, favicon and
   footer per `sw-checklist`, README, LICENSE, COPYRIGHT.
2. `vendor-swtos-image` -- commit `program.bin` and `program.debug.json` with
   recorded provenance, a refresh script, and `include_bytes!` wiring.
3. `vendor-frontend-core` -- copy and adapt `protocol`, `ui`, `resource`, and
   `debug`; verify the whole tree builds for `wasm32-unknown-unknown`.

**Phase 1 -- shell-first (deployable)**

4. `virtual-uart` -- in-process byte-queue transport replacing the pty.
5. `emulator-pump` -- `run_batch` plus the 100 Hz heartbeat on a browser timer,
   with drain-and-reset UART collection.
6. `terminal-grid` -- fixed-size character grid, selectable geometry.
7. `keyboard-input` -- key capture to framed `TTY_INPUT`, Ctrl-A prefix
   interception, Escape and backspace handling.
8. `deploy-shell-demo` -- build `pages/`, verify the live URL.

**Phase 2 -- panes**

9. `application-panes` -- channel open/close/title, `run NAME --tty=new`.
10. `resources-pane` -- `SnapshotAssembler` with generation discipline.
11. `debugger-pane` -- identity, `regs`, and `kill` opcodes.
12. `zoom-and-focus` -- Ctrl-A `1`-`9`, `n`, `s`, `x`, `a`, `z`.
13. `copy-mode-scrollback` -- Ctrl-A `y`, arrows/hjkl, PgUp/PgDn, `g`/`G`, `q`.
14. `preemption-acceptance` -- the dual `cpu-hog` proof in the browser.
15. `polish-and-deploy` -- help overlay, geometry selector, docs, final deploy.

## Forward compatibility: ANSI color (documented, not built)

Colored text is a longer-term goal in the CLI first and the web demo second.
Nothing in this plan builds it, but the choices below are made now so the
migration is additive rather than a rewrite.

### Where color enters

Two escape streams must stay strictly separated, and today they already are:

- **Frontend chrome** -- the three escapes `Desktop::render()` emits
  (`\x1b[H`, per-line `\x1b[K`, `\r\n`). These are an artifact of painting a
  real terminal. The browser must never see them, which is the reason
  `render_grid()` exists.
- **Target content** -- SGR sequences that SWTOS would one day write into a
  virtual TTY, arriving as ordinary `TTY_OUTPUT` payload bytes. This is the
  stream that gains color, and it is per-pane.

Keeping the browser on `render_grid()` from day one means adding color touches
only the pane-content path. The chrome path never needs an interpreter.

### The one data-model decision

`render_grid()` should return a grid of cells, not `char`:

```
struct Cell { ch: char, fg: Color, bg: Color, attrs: Attrs }
```

Every cell is `Color::Default` with empty `attrs` until color ships. The web
renderer emits a bare character run for default-attribute cells and only opens
a styled span when attributes differ from the default, so the DOM cost today is
identical to a plain grid and the renderer needs no change later. Returning
`Vec<Vec<char>>` instead would force both the grid API and the renderer to be
rewritten, which is the migration this section exists to avoid.

### What must change when color ships

- **`Pane` scrollback stores attributed lines, not `&str`.** SGR bytes have to
  be consumed by a per-pane state machine as output arrives, with attributes
  attached to cells. Storing raw escapes in the line buffer would corrupt every
  width calculation: line wrapping, the 1,000-line retention, copy-mode
  `horizontal_offset`, and status-line truncation all index by column, and an
  escape sequence occupies zero columns while occupying many bytes.
- **The parser must be resumable.** A `TTY_OUTPUT` frame can split an escape
  sequence across frames, so the state machine's partial-sequence state lives
  in the `Pane`, matching how `ConnectionDecoder` already retains a trailing
  `A5` across fragmented reads.
- **Unsupported sequences are dropped, not printed.** Anything outside the
  recognized SGR subset is consumed and discarded so it can never leak into the
  grid as garbage characters.
- **One palette table.** A 16-color plus default mapping, expressed once and
  bound to the Catppuccin Mocha variables the page already defines. Bold maps
  to the bright half of the palette rather than to a font weight, so the
  character grid stays monospaced and cell-aligned.

### Sequencing

Color is a longer-term goal in the CLI first and here second, but that ordering
is a preference, not a dependency. **No work in this repo requires or implies a
change to `sw-tos`.** The `Cell` grid lands here whenever this repo is ready,
because `render_grid()` is already this repo's own adaptation of the vendored
`ui.rs` -- upstream `te-rs` keeps its `render()` untouched.

If `te-rs` later grows an SGR parser of its own, re-vendoring picks it up, which
is the routine way this repo tracks upstream. If it does not, this repo writes
its own. Either way the vendored copy is a snapshot that is free to diverge, and
"upstream should change first" is never a reason to block work here.
