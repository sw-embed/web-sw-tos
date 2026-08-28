# web-sw-tos

Browser live demo of the [SWTOS](https://github.com/sw-embed/sw-tos)
terminal: a preemptively multitasking microkernel running on an emulated
[COR24](https://github.com/sw-embed/sw-cor24-emulator) CPU, driven from a
tiled terminal frontend, entirely client-side via WebAssembly.

**Live demo:** <https://sw-embed.github.io/web-sw-tos/>

Part of the [Software Wrighter COR24 Tools Project](https://sw-embed.github.io/web-sw-cor24-demos/#/).

## What this is

The SWTOS command-line demo is three pieces joined by a pty: a host that runs
the COR24 emulator, the pty itself, and `te-rs`, the tiled terminal frontend.
In the browser all three collapse into a single WebAssembly module and the pty
becomes an in-process virtual UART. Everything else keeps its shape, including
the framed transport and the host-driven scheduler heartbeat that makes
preemption work.

The result is a simulated CLI in a web page: a fixed-size character grid that
the SWTOS frontend divides into panes, driven entirely by `Ctrl-A` commands
exactly as on a real terminal.

## Status

Under construction. See [docs/plan.md](docs/plan.md) for the phase order.

- **Phase 0** -- foundation: scaffold, vendored image, vendored frontend core
- **Phase 1** -- an interactive Shell pane over the virtual UART
- **Phase 2** -- application, resources, and debugger panes, zoom, copy-mode
  scrollback, and the preemption proof

## Usage (once Phase 1 lands)

The frontend prefix is `Ctrl-A`. Release it before typing the command; each
command needs its own prefix.

| Prefix command | Action |
|---|---|
| `1` through `9` | Focus a pane by number |
| `n` | Focus the next pane |
| `s` | Add a pane |
| `x` | Close the focused pane |
| `z` | Toggle zoom for the focused pane |
| `y` | Enter or leave copy mode |
| `?` | Toggle help |

In copy mode, navigate with the arrow keys or `hjkl`, page with `PgUp`/`PgDn`,
jump with `g`/`G`, and leave with `q`.

There is deliberately no mouse support, no scrollbars, and no copy/paste. The
demo reproduces a terminal, and layout, focus, zoom, and scrolling all belong
to the frontend rather than to the browser.

## Build

Requires [Trunk](https://trunkrs.dev/) and a Rust toolchain with the
`wasm32-unknown-unknown` target.

```bash
trunk build                # dev build to dist/
./scripts/serve.sh         # dev server on port 9958
./scripts/build-pages.sh   # release build to pages/ for GitHub Pages
```

Quality gates:

```bash
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --all -- --check
sw-checklist
```

The Pages workflow deploys the **committed** `pages/` directory, so run
`build-pages.sh` and stage `pages/` whenever web source changes.

## Sibling repositories

This project expects these as siblings under the same parent directory:

| Repo | Role |
|---|---|
| [`sw-tos`](https://github.com/sw-embed/sw-tos) | Reference implementation and the source of the vendored frontend and image |
| [`sw-cor24-emulator`](https://github.com/sw-embed/sw-cor24-emulator) | `EmulatorCore` (path dependency) |
| [`sw-cor24-isa`](https://github.com/sw-embed/sw-cor24-isa) | COR24 ISA definitions |

`sw-tos` is never modified by this project. The `te-rs` frontend modules and
the prebuilt SWTOS image are vendored copies living here.

## Technology

| Component | Choice |
|---|---|
| Language | Rust (edition 2024) |
| UI framework | Yew 0.21 (CSR) |
| Build tool | Trunk |
| WASM bindings | wasm-bindgen + web-sys |
| Theme | Catppuccin Mocha |
| Emulator | cor24-emulator (path dependency) |
| Frontend | te-rs core, vendored from sw-tos |

## Links

- Blog: [Software Wrighter Lab](https://software-wrighter-lab.github.io/)
- Discord: [Join the community](https://discord.com/invite/Ctzk5uHggZ)
- YouTube: [Software Wrighter](https://www.youtube.com/@SoftwareWrighter)

## Copyright

Copyright (c) 2026 Michael A. Wright

## License

MIT
