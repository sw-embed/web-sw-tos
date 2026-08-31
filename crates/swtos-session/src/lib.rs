//! Pure session logic for the SWTOS browser demo.
//!
//! Nothing here touches `js_sys`, `web_sys`, `gloo`, or `yew`. That is the
//! point: the frame routing, the prefix state machine, and the debugger
//! console are the parts that carry the behaviour worth testing, and while
//! they read the clock directly they could only be exercised in a browser.
//! Time arrives through the [`state::Clock`] trait instead, so every path
//! below is reachable from an ordinary `cargo test`.
//!
//! Data declarations live in [`state`]; behaviour lives in the modules that
//! operate on them, as functions rather than methods hung off the data.

pub mod build;
pub mod debugger;
pub mod driver;
pub mod routing;
pub mod sending;
pub mod state;
