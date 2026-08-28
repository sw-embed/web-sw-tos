# SWTOS -- Live Terminal Demo

A preemptively multitasking microkernel running on an emulated 24-bit RISC
CPU, driven from a tiled terminal frontend, entirely inside your browser.
Rust compiled to WebAssembly, with no server and nothing to install.

**[Live Demo](https://sw-embed.github.io/web-sw-tos/)**

![SWTOS running in the browser](images/screenshot.png?ts=1787946868000)

Part of the [Software Wrighter COR24 Tools Project](https://sw-embed.github.io/web-sw-cor24-demos/#/).

## Introduction

[SWTOS](https://github.com/sw-embed/sw-tos) is a clean-room microkernel for
small CPUs, inspired by MINIX IPC principles. It runs on the MakerLisp COR24
soft CPU -- a 24-bit RISC core for Lattice FPGAs -- and provides synchronous
message-passing IPC, a resident program catalog, and preemptive scheduling
without an MMU, without hardware multiply, and without floating point.

Its command-line demo is three programs joined by a pty: a host process
running the COR24 emulator, the pty itself, and `te-rs`, a tiled terminal
frontend that keeps the shell, applications, a debugger, and a resource
monitor in independent panes.

This project collapses all three into a single WebAssembly module. The pty
becomes an in-process virtual UART, and everything else keeps its shape --
the same framed transport, the same pane model, and the same host-driven
scheduler heartbeat that makes preemption work. The result is a simulated
CLI in a web page: one fixed-size character grid that the frontend divides
into panes, driven entirely by `Ctrl-A` commands exactly as on a real
terminal.

There is deliberately no mouse support, no scrollbars, and no copy/paste.
The demo reproduces a terminal, so layout, focus, zoom, and scrolling belong
to the frontend rather than to the browser.

### Why it is interesting

The headline is preemption. The `cpu-hog` program in the SWTOS catalog
contains no yield, no syscall, no I/O, no IPC, no sleep, and no blocking
operation of any kind. Two copies of it can run at once while the shell
stays interactive and the resource monitor keeps updating -- because the
scheduler is driven by a timer heartbeat the host sends over the UART, and
the kernel preempts a running process from the interrupt context.

Watching that work in a browser tab, against a real emulated CPU rather than
a simulation of one, is the point of this demo.

## Status

**The shell is live and interactive.** SWTOS boots on the emulated CPU, prints
its menu, and responds to your keystrokes. Press `1` for Hello, `2` for
Counter, `5` for the multitasking demo -- the screenshot above shows two
processes interleaving their output (`B1`, `C1`, `B2`, `C2`).

The tiled pane frontend is still being ported.

| Phase | Scope | State |
|---|---|---|
| 0 | Foundation: scaffold, vendored image, vendored transport | done |
| 1 | An interactive Shell over the virtual UART | done |
| 2 | Tiled panes, zoom, copy-mode scrollback, the preemption proof | in progress |

### Performance

The emulator runs about 7x slower in WebAssembly than natively, reaching
roughly 60 scheduler ticks per second against a target of 100. Typing latency
is bounded by one tick and is imperceptible, and preemption is unaffected
because it is driven by heartbeat count rather than wall-clock rate. The one
visible artifact is that the Uptime and Clock programs run slow. See
[docs/plan.md](docs/plan.md) for the measurements.

## Usage

Click into the page and type. Keys go to the target wherever focus sits, so
there is nothing to click first. `Cmd`/`Alt` combinations are left to the
browser, so reload and devtools keep working.

At the `Choice:` prompt, press a digit:

| Key | Program |
|---|---|
| `1` | Hello |
| `2` | Counter |
| `3` | Uptime |
| `4` | Clock |
| `5` | Multitask -- two processes interleaving |
| `6` | UART test |

Once the tiled frontend lands, the frontend prefix is `Ctrl-A`. Release it
before typing the command; each command needs its own prefix.

| Prefix command | Action |
|---|---|
| `1` through `9` | Focus a pane by number |
| `n` | Focus the next pane |
| `s` | Add a pane |
| `c` | Clear the focused pane |
| `x` | Close the focused pane |
| `z` | Toggle zoom for the focused pane |
| `y` | Enter or leave copy mode |
| `?` | Toggle help |

In copy mode, navigate with the arrow keys or `hjkl`, page with `PgUp`/`PgDn`,
jump with `g`/`G`, and leave with `q`.

There is deliberately no mouse support, no scrollbars, and no copy/paste. The
demo reproduces a terminal, and layout, focus, zoom, and scrolling all belong
to the frontend rather than to the browser.

## Documentation

- [docs/plan.md](docs/plan.md) -- architecture, the decisions behind it, the
  vendored-module triage, known hazards, and the phase order
- [docs/architecture.md](docs/architecture.md) -- the workspace and module
  tree, dependency direction, and what is vendored from where

Upstream reference material:

- [SWTOS protocol](https://github.com/sw-embed/sw-tos/blob/main/docs/protocol.md)
  -- the framed multiplexed transport this demo speaks
- [SWTOS preemptive multitasking](https://github.com/sw-embed/sw-tos/blob/main/docs/preemptive-multitasking.md)
  -- the UART-clock scheduling design
- [SWTOS windows frontend](https://github.com/sw-embed/sw-tos/blob/main/docs/windows-usage.md)
  -- the tiled frontend this demo ports

## Development

### Requirements

- A Rust toolchain with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev/)
- These sibling repositories checked out beside this one:

| Repo | Role |
|---|---|
| [`sw-tos`](https://github.com/sw-embed/sw-tos) | Reference implementation, and the source of the vendored frontend and system image |
| [`sw-cor24-emulator`](https://github.com/sw-embed/sw-cor24-emulator) | `EmulatorCore`, a path dependency |
| [`sw-cor24-isa`](https://github.com/sw-embed/sw-cor24-isa) | COR24 ISA definitions |

```bash
git clone git@github.com:sw-embed/sw-cor24-emulator.git
git clone git@github.com:sw-embed/sw-cor24-isa.git
rustup target add wasm32-unknown-unknown
```

`sw-tos` is never modified by this project. The `te-rs` frontend modules and
the prebuilt system image are vendored copies living here, free to diverge;
re-vendoring is how this repo tracks upstream.

### Build and run

```bash
trunk build                # dev build to dist/
./scripts/serve.sh         # dev server on port 9958
./scripts/build-pages.sh   # release build to pages/ for GitHub Pages
```

### Quality gates

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
sw-checklist
```

Clippy runs clean with zero warnings; warnings are fixed, never suppressed.
`sw-checklist` is expected to stay at 16 passed, 0 failed, 0 warnings.

### Deployment

The Pages workflow deploys the **committed** `pages/` directory rather than
building in CI, so the WebAssembly bundle is built locally and checked in.
Run `./scripts/build-pages.sh` and stage `pages/` whenever web source
changes.

### Project layout

```
src/            Yew application
crates/         swtos-frontend (vendored te-rs core), swtos-host (emulator side)
assets/         vendored SWTOS image and debug map
docs/           design documents
images/         README assets
pages/          committed GitHub Pages output
scripts/        serve.sh, build-pages.sh
```

## Links

- Blog: [Software Wrighter Lab](https://software-wrighter-lab.github.io/)
- Discord: [Join the community](https://discord.com/invite/Ctzk5uHggZ)
- YouTube: [Software Wrighter](https://www.youtube.com/@SoftwareWrighter)
- Hardware: [MakerLisp COR24 Test Board](https://www.makerlisp.com/cor24-test-board)

## License

MIT License - see [LICENSE](LICENSE)

Copyright (c) 2026 Michael A Wright
