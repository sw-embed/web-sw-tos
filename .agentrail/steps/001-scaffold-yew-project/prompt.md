Scaffold the Yew/Rust/WASM project for web-sw-tos. Read docs/plan.md first
(Phase 0, step 1) and CLAUDE.md. Use ../web-sw-cor24-apl as the shape
reference for BUILD conventions only -- do not copy its branching policy.

Deliverables:

- Cargo.toml: edition 2024, crate-type ["cdylib", "rlib"], deps yew 0.21
  (csr), wasm-bindgen, web-sys, js-sys, gloo, console_error_panic_hook, and
  path deps cor24-emulator = { path = "../sw-cor24-emulator" } and
  cor24-isa = { path = "../sw-cor24-isa", features = ["serde"] }.
  Release profile opt-level = "z", lto = true.
- index.html: Trunk entry with the Catppuccin Mocha palette, a favicon, and
  a footer (both are sw-checklist Web UI requirements). The page frames one
  FIXED-SIZE character grid -- no responsive reflow of the grid itself, and
  no scrollbars on it.
- src/main.rs (Yew renderer entry) and src/lib.rs (App component, module
  declarations). Keep sw-checklist's limits in view from the start:
  functions <= 25 LOC, modules <= 4 functions, crates <= 4 modules. This
  project will exceed 4 modules, so plan the module tree deliberately and
  record the intended shape in docs/ rather than letting it sprawl.
- src/app.css.
- scripts/serve.sh and scripts/build-pages.sh (the latter must use
  --public-url /web-sw-tos/ and rsync dist/ -> pages/ with .nojekyll kept).
- .github/workflows/pages.yml deploying the committed pages/ directory.
- .gitignore (target/, dist/, .DS_Store), LICENSE (MIT), COPYRIGHT
  (Copyright (c) 2026 Michael A. Wright), and a README.md describing the
  demo, the build, and the technology table.

Acceptance:

- `cargo build --target wasm32-unknown-unknown` succeeds. This is the step
  that PROVES the emulator crate is wasm-clean; it reaches std::fs in its SPI
  peripherals and SystemTime::now() in the I2C registry. If either breaks the
  build or would panic on a path SWTOS touches, STOP and record the finding
  in docs/plan.md under known hazards before proceeding -- do not paper over
  it. Note that this is a build check only; run_batch is not exercised until
  the emulator-pump step.
- `trunk build` succeeds and produces dist/.
- `cargo clippy --all-targets --all-features -- -D warnings` is clean.
- `cargo fmt --all -- --check` is clean.
- Record the sw-checklist before/after counts for the commit message.

Do NOT vendor te-rs modules or the SWTOS image in this step -- those are
steps 2 and 3.
