//! Dismissing the help overlay.
//!
//! The overlay itself says "close help: q, Escape, or ?", and te-rs honours
//! that by routing those three straight to the command table while help is
//! open, without the prefix. A frontend that demands the prefix first leaves the
//! overlay stuck with no documented way out.

use swtos_input::dispatch;
use swtos_session::driver;
use swtos_session::state::Session;

fn open_help() -> Session {
    let mut session = driver::session();
    dispatch::key(&mut session, dispatch::PREFIX_KEY, true);
    dispatch::key(&mut session, "?", false);
    assert!(
        session.panes.desktop.help_enabled(),
        "prefix-? did not open help"
    );
    session
}

#[test]
fn every_documented_key_closes_help() {
    for key in ["q", "Escape", "?"] {
        let mut session = open_help();
        assert!(
            dispatch::key(&mut session, key, false).is_empty(),
            "{key} leaked to the target instead of closing help"
        );
        assert!(
            !session.panes.desktop.help_enabled(),
            "{key} did not close help, though the overlay says it does"
        );
    }
}

#[test]
fn the_prefix_still_closes_help() {
    let mut session = open_help();
    dispatch::key(&mut session, dispatch::PREFIX_KEY, true);
    dispatch::key(&mut session, "?", false);
    assert!(
        !session.panes.desktop.help_enabled(),
        "prefix-? did not toggle help off"
    );
}

/// Keys the overlay does not claim must not be swallowed by it.
#[test]
fn other_keys_are_unaffected_while_help_is_open() {
    let mut session = open_help();
    assert_eq!(
        dispatch::key(&mut session, "x", false),
        b"x",
        "an ordinary key was eaten by the help overlay"
    );
}
