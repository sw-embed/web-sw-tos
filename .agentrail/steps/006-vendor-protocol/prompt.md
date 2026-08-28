Vendor ONLY the framed-transport module from te-rs. This is the risk-first split: protocol.rs is the one vendored module the byte path needs, so it lands before the pump, while ui.rs / resource.rs / debug.rs stay in the later vendor-frontend-core step (they serve the display, not the transport).

Source (READ ONLY, never modify): ../sw-tos/tools/te-rs/src/protocol.rs. Per docs/plan.md's triage this module is pure -- no std::fs, no Instant, no libc -- so it should drop in essentially unchanged. Its tests come with it; keep them.

Deliverables:

- crates/swtos-frontend/ as a new workspace member (add to [workspace] members in the root Cargo.toml at the same time).
- crates/swtos-frontend/src/protocol.rs, vendored, plus src/lib.rs.
- A header comment on the vendored file naming the source repo, path, and commit it came from, so re-vendoring later is unambiguous.
- serde is a dependency only if the vendored code actually needs it; do not carry dependencies the module does not use.

Acceptance: the vendored tests pass unchanged (cargo test -p swtos-frontend); cargo build --workspace --target wasm32-unknown-unknown succeeds; clippy -D warnings and fmt clean; sw-checklist must not regress from 21 passed / 0 failed / 0 warnings (the new crate will add its own checks -- all of them must pass too).