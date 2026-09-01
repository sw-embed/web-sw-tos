//! Where a key goes: frontend command, local console, or the target.
//!
//! Four consumers sit in front of the target, in order. A bare modifier is
//! dropped. A pending prefix makes this key a frontend command. Ctrl-A arms
//! that prefix. Copy mode claims navigation. Only what none of them wants
//! reaches SWTOS.

use crate::restart;
use crate::translate;
use swtos_frontend::ui::PaneKind;
use swtos_session::routing::SHELL;
use swtos_session::state::Session;
use swtos_session::{debugger, sending};

/// Keys the help overlay claims for itself, taken without the prefix because
/// the overlay tells the reader to press exactly these.
const DISMISS_HELP: [&str; 3] = ["q", "Escape", "?"];

/// Handle one key. Returns whatever was queued for the target, empty when the
/// frontend consumed it.
pub fn key(session: &mut Session, key: &str, ctrl: bool) -> Vec<u8> {
    let (key, ctrl) = translate::normalise(key, ctrl);
    let key = key.as_str();
    if let Some(consumed) = prefix(session, key, ctrl) {
        return consumed;
    }
    // The overlay says "close help: q, Escape, or ?", so those three reach the
    // command table directly while it is open. Requiring the prefix here left
    // the overlay with no documented way out at all.
    if session.panes.desktop.help_enabled() && DISMISS_HELP.contains(&key) {
        session.panes.desktop.command(translate::command_byte(key));
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

/// The prefix state machine: drop bare modifiers, run a pending command, or
/// arm on Ctrl-A. `Some` means the key was consumed here.
///
/// A bare modifier keydown is neither a command nor input. Without dropping
/// it, it is consumed as the prefix command, so every binding needing Shift --
/// `?` for help, `S` to restore a pane -- is swallowed by the Shift that
/// produces it. A terminal never sees this; only a browser does.
fn prefix(session: &mut Session, key: &str, ctrl: bool) -> Option<Vec<u8>> {
    if translate::is_modifier(key) {
        return Some(Vec::new());
    }
    if core::mem::take(&mut session.input.prefix_armed) {
        return Some(command(session, key));
    }
    if ctrl && key.eq_ignore_ascii_case("a") {
        session.input.prefix_armed = true;
        return Some(Vec::new());
    }
    None
}

/// Run one frontend command. Two reach the target rather than the desktop:
/// `e` sends a running application a real Escape, and `k` asks for the shell
/// to be restarted -- the only way out of a command that will not give the
/// CPU back.
fn command(session: &mut Session, key: &str) -> Vec<u8> {
    if key == "e" {
        let channel = session.panes.desktop.focused_channel();
        sending::to_channel(session, channel, &[0x1b]);
        return vec![0x1b];
    }
    if key == "k" {
        restart::send(session);
        return Vec::new();
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
                // `!kill 1` is the shell. Asking the shell to kill itself by
                // typing at it needs the shell to be reading, which is the
                // thing in doubt whenever this is asked for.
                if restart::is_request(&line) {
                    restart::send(session);
                } else {
                    sending::to_channel(session, SHELL, format!("{line}\n").as_bytes());
                }
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
