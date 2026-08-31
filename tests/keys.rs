//! Browser key events to terminal bytes, frontend commands, and copy mode.
//!
//! Every case here fails silently when it is wrong: a key that produces no
//! bytes is indistinguishable from an emulator that is not running.

use web_sw_tos::keys;
use web_sw_tos::session::Session;

fn rendered(session: &Session) -> String {
    session
        .grid(80, 24)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn control_keys_use_their_terminal_encodings() {
    assert_eq!(
        keys::to_bytes("Enter", false),
        b"\n",
        "the shell terminates on LF, not CR"
    );
    assert_eq!(keys::to_bytes("Backspace", false), vec![0x08]);
    assert_eq!(keys::to_bytes("Tab", false), b"\t");
    assert_eq!(keys::to_bytes("Escape", false), vec![0x1b]);
}

#[test]
fn printable_and_ctrl_letters_map_as_a_terminal_would() {
    assert_eq!(keys::to_bytes("1", false), b"1");
    assert_eq!(
        keys::to_bytes("a", true),
        vec![0x01],
        "Ctrl-A is the prefix"
    );
    assert_eq!(keys::to_bytes("A", true), vec![0x01]);
    assert_eq!(keys::to_bytes("z", true), vec![0x1a]);
    for named in ["Shift", "ArrowUp", "F5", "CapsLock"] {
        assert!(keys::to_bytes(named, false).is_empty(), "{named} leaked");
    }
}

/// Ctrl-A must never reach the target: it arms the frontend instead.
#[test]
fn the_prefix_is_consumed_and_the_next_key_is_a_command() {
    let mut session = Session::default();
    assert!(
        session.send_key("a", true).is_empty(),
        "prefix leaked to target"
    );
    assert!(session.status().prefix_armed, "prefix did not arm");
    assert!(
        session.send_key("z", false).is_empty(),
        "command leaked to target"
    );
    assert!(!session.status().prefix_armed, "prefix stayed armed");
}

/// Prefix-e is the only way to send Escape to a running application, which is
/// how Uptime and Clock are stopped.
#[test]
fn prefix_e_sends_escape_to_the_target() {
    let mut session = Session::default();
    session.send_key("a", true);
    assert_eq!(session.send_key("e", false), vec![0x1b]);
}

/// Without the prefix, a bare key is ordinary input.
#[test]
fn an_unprefixed_key_reaches_the_target() {
    let mut session = Session::default();
    assert_eq!(session.send_key("3", false), b"3");
}

/// The target never echoes, so the frontend does -- but control bytes must be
/// filtered, or a raw Escape renders as a replacement character in the pane.
#[test]
fn echo_shows_typing_but_not_control_bytes() {
    let mut session = Session::default();
    for key in ["h", "i", "x", "Backspace", "Enter"] {
        session.send_key(key, false);
    }
    session.send_key("Escape", false);
    let screen = rendered(&session);
    assert!(
        screen.contains("hi"),
        "typing never reached the screen: {screen}"
    );
    assert!(!screen.contains("hix"), "backspace did not erase");
    assert!(
        !screen.contains('\u{fffd}'),
        "a control byte was echoed into the pane: {screen}"
    );
}

/// Copy mode claims navigation keys so scrolling does not type into the shell.
#[test]
fn copy_mode_claims_navigation_and_releases_it_on_exit() {
    let mut session = Session::default();
    session.send_key("a", true);
    session.send_key("y", false);
    assert!(
        session.send_key("k", false).is_empty(),
        "copy mode let a motion key through to the target"
    );
    session.send_key("q", false);
    assert_eq!(session.send_key("k", false), b"k", "copy mode never exited");
}

/// docs/use-cases.md binds clear to `Ctrl-A l`; `c` shipped here first and is
/// kept as an alias.
#[test]
fn both_clear_bindings_are_consumed_by_the_frontend() {
    for key in ["l", "c"] {
        let mut session = Session::default();
        session.send_key("a", true);
        assert!(
            session.send_key(key, false).is_empty(),
            "Ctrl-A {key} leaked to the target instead of clearing"
        );
    }
}

/// A modifier keydown must not consume the prefix.
///
/// `?` and `S` are both produced by holding Shift, and the browser delivers
/// that Shift as its own keydown first. Treating it as the prefix command
/// swallowed it, leaving `Ctrl-A ?` and `Ctrl-A S` dead while unshifted
/// bindings like `Ctrl-A h` worked -- which is exactly how it was reported.
#[test]
fn a_shift_press_does_not_swallow_the_prefix() {
    for modifier in ["Shift", "Control", "Alt", "Meta", "CapsLock", "AltGraph"] {
        let mut session = Session::default();
        session.send_key("a", true);
        assert!(
            session.send_key(modifier, false).is_empty(),
            "{modifier} leaked to the target"
        );
        assert!(
            session.status().prefix_armed,
            "{modifier} consumed the armed prefix"
        );
        // The real command still lands afterwards.
        assert!(session.send_key("?", false).is_empty());
        assert!(!session.status().prefix_armed, "the command never ran");
    }
}

/// A modifier on its own is not input either.
#[test]
fn a_modifier_alone_sends_nothing() {
    let mut session = Session::default();
    assert!(session.send_key("Shift", false).is_empty());
}
