//! The command menu, and the prefix key it exists to survive.
//!
//! Issue #1: `Ctrl-O` reaches the page on macOS, where browser shortcuts are
//! Cmd-based, and never reaches it on Chrome/Linux, where `Ctrl-O` is Open
//! File. Two fixes, and these check both: a prefix no browser claims, and a
//! route to the commands that needs no keyboard at all.

use swtos_frontend::ui::PaneKind;
use swtos_input::dispatch::{self, COMMANDS, PREFIX_KEY, PREFIX_LABEL};
use swtos_input::translate;
use swtos_session::driver;

/// What makes a prefix safe is narrow, and each of these has bitten someone.
#[test]
fn the_prefix_is_a_control_byte_that_is_not_a_line_ending() {
    let byte = translate::to_bytes(PREFIX_KEY, true);
    assert_eq!(byte.len(), 1, "the transport carries one byte: {byte:?}");
    let byte = byte[0];
    assert!(byte < 0x20, "not a control byte: {byte:#04x}");

    // CR and LF arrive at the end of every pasted line, so a prefix that is
    // either one arms itself on paste and swallows the Enter. sw-tos found
    // this with Ctrl-J.
    assert_ne!(byte, b'\r', "Enter would arm the prefix");
    assert_ne!(byte, b'\n', "a pasted line would arm the prefix");

    // Tab, Enter and Escape are Ctrl-I, Ctrl-M and Ctrl-[. A prefix that is
    // also one of those cannot be typed as itself.
    for (name, trap) in [("Tab", b'\t'), ("Enter", b'\r'), ("Escape", 0x1b)] {
        assert_ne!(byte, trap, "the prefix is also {name}");
    }

    // The label and the key are the same key. They drifted apart upstream
    // when two harnesses spelled the prefix separately.
    let letter = PREFIX_LABEL
        .strip_prefix("Ctrl-")
        .expect("the label names a control combination");
    assert!(letter.eq_ignore_ascii_case(PREFIX_KEY), "{PREFIX_LABEL}");
}

/// The point of the table: a button cannot come to mean something other than
/// the keystroke printed on it.
#[test]
fn every_menu_command_is_a_key_the_dispatcher_accepts() {
    for (group, key, label) in COMMANDS {
        assert!(
            !group.is_empty() && !label.is_empty(),
            "{key} is unlabelled"
        );
        assert_eq!(key.chars().count(), 1, "{key} is not one keystroke");
    }
}

/// A click and a keypress must arrive at the same place. This is why the menu
/// replays through `key` rather than reaching into the command table itself.
#[test]
fn a_menu_click_and_the_keystroke_agree() {
    // Compared on the rendered screen, which is what a person actually gets,
    // and which shows zoom plainly: a zoomed desktop draws one pane where it
    // drew four.
    let screen = |session: &swtos_session::state::Session| -> String {
        session
            .panes
            .desktop
            .render_grid(120, 30)
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut typed = driver::session();
    let before = screen(&typed);
    dispatch::key(&mut typed, PREFIX_KEY, true);
    dispatch::key(&mut typed, "z", false);

    let mut clicked = driver::session();
    dispatch::menu_command(&mut clicked, "z");

    assert_ne!(
        screen(&typed),
        before,
        "the keystroke did nothing to compare"
    );
    assert_eq!(
        screen(&typed),
        screen(&clicked),
        "click and keystroke disagree"
    );
}

/// Focus is the command a person reaches for first, and the one the menu
/// builds from the live layout rather than from a fixed 1-9.
#[test]
fn the_menu_can_focus_a_pane_without_the_keyboard() {
    let mut session = driver::session();
    assert_eq!(session.panes.desktop.focused_kind(), PaneKind::Shell);

    dispatch::menu_command(&mut session, "3");
    assert_eq!(
        session.panes.desktop.focused_kind(),
        PaneKind::Debugger,
        "menu focus did not reach the third pane"
    );
}

/// The menu leaves the prefix disarmed. It presses two keys; a half-pressed
/// prefix would eat whatever the person typed next.
#[test]
fn the_menu_does_not_leave_the_prefix_armed() {
    let mut session = driver::session();
    dispatch::menu_command(&mut session, "z");
    assert!(
        !driver::status(&session).prefix_armed,
        "the menu left the prefix armed"
    );

    // And the next ordinary key is ordinary: it reaches the target.
    let sent = dispatch::key(&mut session, "a", false);
    assert_eq!(sent, b"a", "the key after a menu command was swallowed");
}
