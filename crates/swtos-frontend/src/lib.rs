//! Vendored core of the `te-rs` SWTOS terminal frontend.
//!
//! These modules are **copies** taken from `sw-tos/tools/te-rs/src/`, which is
//! read-only to this project. They are free to diverge; re-vendoring is how
//! this crate tracks upstream, and every divergence is marked in the file that
//! carries it. See `docs/plan.md` for the per-module triage.

pub mod debug;
pub mod protocol;
pub mod resource;
pub mod ui;
