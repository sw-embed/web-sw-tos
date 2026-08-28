//! Browser key events to terminal bytes.
//!
//! This mapping is the whole of the demo's input path, and every case here is
//! one that silently does nothing if it is wrong -- a key that sends no bytes
//! looks identical to an emulator that is not running.

use web_sw_tos::session::Session;

#[test]
fn control_keys_use_their_terminal_encodings() {
    let mut session = Session::default();
    assert_eq!(session.send_key("Enter", false), b"\r", "Enter must be CR");
    assert_eq!(session.send_key("Backspace", false), vec![0x08]);
    assert_eq!(session.send_key("Tab", false), b"\t");
    assert_eq!(session.send_key("Escape", false), vec![0x1b]);
}

#[test]
fn printable_keys_pass_through() {
    let mut session = Session::default();
    assert_eq!(session.send_key("1", false), b"1");
    assert_eq!(session.send_key("z", false), b"z");
    assert_eq!(session.send_key(" ", false), b" ");
}

/// Ctrl-A is the frontend prefix once panes exist, so its encoding has to be
/// right well before anything depends on it.
#[test]
fn ctrl_letters_collapse_to_control_codes() {
    let mut session = Session::default();
    assert_eq!(session.send_key("a", true), vec![0x01]);
    assert_eq!(session.send_key("A", true), vec![0x01]);
    assert_eq!(session.send_key("c", true), vec![0x03]);
    assert_eq!(session.send_key("z", true), vec![0x1a]);
}

/// Named keys are longer than one character. Sending them as text would type
/// the literal word "Shift" into the shell.
#[test]
fn named_keys_send_nothing() {
    let mut session = Session::default();
    for key in ["Shift", "Control", "ArrowUp", "F5", "CapsLock"] {
        assert!(session.send_key(key, false).is_empty(), "{key} leaked");
    }
    assert!(session.send_key("1", true).is_empty(), "Ctrl-digit leaked");
}

/// The target never echoes, so the frontend does. Checked against the
/// rendered grid rather than an internal buffer, because what matters is that
/// typing reaches the visible screen: backspace must erase, and Enter must
/// commit the line rather than restart it the way a pane treats a bare CR.
#[test]
fn typing_is_echoed_onto_the_screen() {
    let mut session = Session::default();
    for key in ["h", "i", "x", "Backspace", "Enter"] {
        session.send_key(key, false);
    }
    let screen = rendered(&session);
    assert!(
        screen.contains("hi"),
        "typing never reached the screen: {screen}"
    );
    assert!(!screen.contains("hix"), "backspace did not erase: {screen}");
}

/// A key the mapping ignores must not reach the screen either.
#[test]
fn ignored_keys_leave_no_trace() {
    let mut session = Session::default();
    session.send_key("ArrowUp", false);
    session.send_key("Shift", false);
    assert!(!rendered(&session).contains("Arrow"));
    assert!(!rendered(&session).contains("Shift"));
}

fn rendered(session: &Session) -> String {
    session
        .grid(80, 24)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
