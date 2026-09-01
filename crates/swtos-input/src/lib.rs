//! Keyboard handling for the SWTOS browser demo.
//!
//! Separate from the session because translation and dispatch are one job and
//! the session's byte path is another: nothing here knows what a frame is, and
//! nothing in the session knows what a keydown is. The layering runs one way,
//! this crate onto `swtos-session`, never back.

pub mod dispatch;
pub mod recovery;
pub mod translate;
