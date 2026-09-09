//! Where a key goes: frontend command, local console, or the target.
//!
//! Four consumers sit in front of the target, in order. A bare modifier is
//! dropped. A pending prefix makes this key a frontend command. The prefix key
//! arms that prefix. Copy mode claims navigation. Only what none of them wants
//! reaches SWTOS.

use crate::recovery;
use crate::translate;
use swtos_frontend::ui::PaneKind;
use swtos_session::routing::SHELL;
use swtos_session::state::Session;
use swtos_session::{debugger, sending};

/// The host-command prefix, named once.
///
/// `Ctrl-B`, which is tmux's own prefix, and **not** upstream's `Ctrl-O`. That
/// divergence is forced by the browser, the same way `Instant` and `std::fs`
/// are: on macOS the browser's shortcuts are Cmd-based so every Ctrl
/// combination falls through to the page, but on Linux and Windows they are
/// Ctrl-based and `Ctrl-O` is Open File -- taken by the browser before the
/// page sees the keydown, where `preventDefault` cannot reach it. The whole
/// command set was dead on those platforms (issue #1).
///
/// What makes a prefix safe here is narrow. It must be a control byte, since
/// the transport carries one byte; it must be neither CR nor LF, which arrive
/// at the end of every pasted line; and no browser may claim it first.
/// `Ctrl-B` is 0x02, is neither line ending, and is bound by no browser except
/// Firefox's bookmarks sidebar, which a page may cancel. Tab, Enter and Escape
/// are `Ctrl-I`, `Ctrl-M` and `Ctrl-[`, so those three are traps of the same
/// kind and are not candidates.
///
/// A key can still be lost to a platform nobody tested, which is why the
/// command menu exists: it needs no keyboard at all.
pub const PREFIX_KEY: &str = "b";

/// How to say it. The label the help overlay prints comes from here, so the
/// screen cannot name a different key from the one that works -- which is
/// exactly how upstream's two test harnesses drifted apart.
pub const PREFIX_LABEL: &str = "Ctrl-B";

/// The prefix commands worth offering to a mouse, as `(group, key, label)`.
///
/// Data, and the only list of its kind: the menu renders from here and clicks
/// are replayed through [`key`], so a button cannot come to mean something
/// different from the keystroke it names. Pane focus is absent on purpose --
/// the numbers depend on which panes exist, so the menu builds those from the
/// live layout instead of guessing here.
pub const COMMANDS: [(&str, &str, &str); 12] = [
    ("Panes", "n", "next"),
    ("Panes", "p", "previous"),
    ("Panes", "z", "zoom"),
    ("Panes", "s", "split"),
    ("Panes", "x", "close"),
    ("Panes", "c", "close ended"),
    ("View", "l", "clear pane"),
    ("View", "S", "restore panes"),
    ("View", "y", "copy mode"),
    ("Session", "k", "restart shell"),
    ("Session", "B", "warm reboot"),
    ("Session", "?", "help"),
];

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
/// arm on the prefix key. `Some` means the key was consumed here.
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
    if ctrl && key.eq_ignore_ascii_case(PREFIX_KEY) {
        session.input.prefix_armed = true;
        return Some(Vec::new());
    }
    None
}

/// Run one frontend command. Three reach the target rather than the desktop:
/// `e` sends a running application a real Escape, `k` asks for the shell to be
/// restarted -- the only way out of a command that will not give the CPU back
/// -- and `B` asks for a warm reboot when a restart is not enough.
fn command(session: &mut Session, key: &str) -> Vec<u8> {
    if key == "e" {
        let channel = session.panes.desktop.focused_channel();
        sending::to_channel(session, channel, &[0x1b]);
        return vec![0x1b];
    }
    if key == "k" {
        recovery::restart_shell(session);
        return Vec::new();
    }
    if key == "B" {
        recovery::reboot_system(session);
        return Vec::new();
    }
    session.panes.desktop.command(translate::command_byte(key));
    Vec::new()
}

/// Run a prefix command the way the menu asks for it: exactly as if the prefix
/// key had been pressed and then this one.
///
/// Two calls through [`key`] rather than a reach into [`command`], so a click
/// and a keystroke cannot diverge -- bare modifiers, the help overlay and copy
/// mode all get their say in the same order either way.
pub fn menu_command(session: &mut Session, key_name: &str) {
    key(session, PREFIX_KEY, true);
    key(session, key_name, false);
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
                if recovery::is_shell_restart(&line) {
                    recovery::restart_shell(session);
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
