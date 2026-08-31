//! Where a key goes: frontend command, local console, or the target.
//!
//! Four consumers sit in front of the target, in order. A bare modifier is
//! dropped. A pending prefix makes this key a frontend command. Ctrl-A arms
//! that prefix. Copy mode claims navigation. Only what none of them wants
//! reaches SWTOS.

use crate::translate;
use swtos_frontend::ui::PaneKind;
use swtos_session::routing::SHELL;
use swtos_session::state::Session;
use swtos_session::{debugger, sending};

/// Handle one key. Returns whatever was queued for the target, empty when the
/// frontend consumed it.
pub fn key(session: &mut Session, key: &str, ctrl: bool) -> Vec<u8> {
    // A bare modifier keydown is neither a command nor input. Without this it
    // is consumed as the prefix command, so every binding needing Shift --
    // `?` for help, `S` to restore a pane -- is swallowed by the Shift that
    // produces it. A terminal never sees this; only a browser does.
    if translate::is_modifier(key) {
        return Vec::new();
    }
    if core::mem::take(&mut session.input.prefix_armed) {
        return command(session, key);
    }
    if ctrl && key.eq_ignore_ascii_case("a") {
        session.input.prefix_armed = true;
        return Vec::new();
    }
    if session.panes.desktop.copy_mode_enabled()
        && translate::copy_motion(&mut session.panes.desktop, key)
    {
        return Vec::new();
    }
    if console_pane(session, key) {
        return Vec::new();
    }
    to_target(session, key, ctrl)
}

/// Run one frontend command. Prefix-`e` is the exception that reaches the
/// target: it is how a running application is sent a real Escape.
fn command(session: &mut Session, key: &str) -> Vec<u8> {
    if key == "l" || key == "c" {
        session.panes.desktop.clear_focused();
        return Vec::new();
    }
    if key == "e" {
        let channel = session.panes.desktop.focused_channel();
        sending::to_channel(session, channel, &[0x1b]);
        return vec![0x1b];
    }
    session.panes.desktop.command(translate::command_byte(key));
    Vec::new()
}

/// Give a local-console pane first refusal. The Debugger and Resources panes
/// are consoles, not terminals: their channels have no TTY on the target, so
/// routing their keys out as TTY_INPUT discards them silently.
fn console_pane(session: &mut Session, key: &str) -> bool {
    match session.panes.desktop.focused_kind() {
        PaneKind::Debugger => {
            if let Some(request) =
                debugger::key(&mut session.console, &mut session.panes.desktop, key)
            {
                sending::debug_request(session, request);
            }
            if let Some(line) = session.console.pending.take() {
                sending::to_channel(session, SHELL, format!("{line}\n").as_bytes());
            }
            true
        }
        PaneKind::Resources => true,
        _ => false,
    }
}

/// Ordinary input: echoed locally because SWTOS never echoes, then sent.
fn to_target(session: &mut Session, key: &str, ctrl: bool) -> Vec<u8> {
    let bytes = translate::to_bytes(key, ctrl);
    let channel = session.panes.desktop.focused_channel();
    let echo = translate::echo_bytes(&bytes);
    session.panes.desktop.push_channel(channel, &echo);
    sending::to_channel(session, channel, &bytes);
    bytes
}
