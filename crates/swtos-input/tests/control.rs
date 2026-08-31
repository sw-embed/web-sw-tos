//! Control-key encodings a terminal produces and a browser does not.
//!
//! A terminal turns Ctrl with `@` through `_` into the matching control code,
//! so `Ctrl-[` and Escape are the same byte and every place that accepts one
//! accepts the other for free. A browser reports `key: "["` with `ctrlKey`
//! instead, so the frontend has to do that conversion itself.

use swtos_input::{dispatch, translate};
use swtos_session::build;

#[test]
fn ctrl_bracket_is_escape() {
    assert_eq!(
        translate::to_bytes("[", true),
        vec![0x1b],
        "Ctrl-[ must produce the same byte as Escape"
    );
    assert_eq!(translate::to_bytes("Escape", false), vec![0x1b]);
}

/// The whole `@`..`_` range, as a terminal encodes it.
#[test]
fn ctrl_covers_the_full_control_range() {
    for (key, byte) in [
        ("@", 0x00u8),
        ("a", 0x01),
        ("z", 0x1a),
        ("[", 0x1b),
        ("\\", 0x1c),
        ("]", 0x1d),
        ("^", 0x1e),
        ("_", 0x1f),
    ] {
        assert_eq!(
            translate::to_bytes(key, true),
            vec![byte],
            "Ctrl-{key} should encode as {byte:#04x}"
        );
    }
}

/// Ctrl-[ closes the help overlay, exactly as Escape does.
#[test]
fn ctrl_bracket_closes_help() {
    let mut session = build::session();
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "?", false);
    assert!(session.panes.desktop.help_enabled(), "help did not open");

    assert!(dispatch::key(&mut session, "[", true).is_empty());
    assert!(
        !session.panes.desktop.help_enabled(),
        "Ctrl-[ did not close help, though Escape does"
    );
}

/// And leaves copy mode, again as Escape does.
#[test]
fn ctrl_bracket_leaves_copy_mode() {
    let mut session = build::session();
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "y", false);
    assert!(
        session.panes.desktop.copy_mode_enabled(),
        "copy mode did not start"
    );

    dispatch::key(&mut session, "[", true);
    assert!(
        !session.panes.desktop.copy_mode_enabled(),
        "Ctrl-[ did not leave copy mode"
    );
}
