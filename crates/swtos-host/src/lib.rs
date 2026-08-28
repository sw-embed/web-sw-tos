//! Emulator-side host for the SWTOS browser demo.
//!
//! This crate is the browser's counterpart to `tools/cor24-debug-adapter` in
//! the `sw-tos` repository: it owns the system image, the virtual UART that
//! replaces the pty, and the pump that drives the emulator.

pub mod image;
pub mod uart;
