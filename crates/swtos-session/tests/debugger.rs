//! The Debugger pane is a local console, not a TTY.
//!
//! Its channel (254) has no terminal on the target, so anything sent there as
//! TTY_INPUT is silently discarded -- the exact symptom of "typing help in the
//! debugger does nothing". These pin that the console handles the keys itself.

use swtos_frontend::ui::Desktop;
use swtos_session::debugger;
use swtos_session::state::Console;

fn type_line(console: &mut Console, desktop: &mut Desktop, line: &str) -> Option<Vec<u8>> {
    for ch in line.chars() {
        debugger::key(console, desktop, &ch.to_string());
    }
    debugger::key(console, desktop, "Enter")
}

/// Render the Debugger pane zoomed to the full screen. Unzoomed it is half
/// the width, and clipping would be a rendering artefact rather than anything
/// about the console.
///
/// A realistic width now: upstream broke the help into grouped lines under
/// seventy characters each, so this no longer has to render wider than any
/// real screen to keep the tail of one long line on it.
fn screen(desktop: &mut Desktop) -> String {
    desktop.command(b'3');
    desktop.command(b'z');
    let text = render(desktop, 120, 50);
    desktop.command(b'z');
    text
}

fn render(desktop: &Desktop, cols: usize, rows: usize) -> String {
    desktop
        .render_grid(cols, rows)
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn help_prints_the_command_list_into_the_pane() {
    let (mut console, mut desktop) = (debugger::console(), Desktop::default());
    assert!(type_line(&mut console, &mut desktop, "help").is_none());
    let text = screen(&mut desktop);
    assert!(text.contains("regs"), "help did not list regs: {text}");
    // Upstream moved killing out of the debugger's own command set: it is now
    // `!kill <ep>`, handed to the shell. See docs/use-cases.md.
    assert!(
        text.contains('!'),
        "help did not advertise shell passthrough: {text}"
    );
}

/// `regs 2` must produce a DEBUG_REQUEST for the target, not screen text.
#[test]
fn regs_produces_a_request_for_the_target() {
    let (mut console, mut desktop) = (debugger::console(), Desktop::default());
    let request = type_line(&mut console, &mut desktop, "regs 2");
    assert_eq!(
        request,
        Some(vec![2, 2]),
        "regs 2 did not request endpoint 2"
    );
}

/// Symbolic commands need the debug map, which is deliberately not compiled
/// into the bundle. They must say so rather than appear broken.
#[test]
fn symbolic_commands_report_the_missing_map() {
    let (mut console, mut desktop) = (debugger::console(), Desktop::default());
    type_line(&mut console, &mut desktop, "dis 1000");
    let text = screen(&mut desktop);
    assert!(
        text.to_lowercase().contains("map"),
        "dis gave no hint that the debug map is absent: {text}"
    );
}

/// `!command` in the debugger runs a shell command, per docs/use-cases.md
/// (`!ps -l`, `!bg mon`, `!kill 3`). It must be handed to the shell rather
/// than parsed as a debugger command.
#[test]
fn a_bang_line_is_handed_to_the_shell() {
    let (mut console, mut desktop) = (debugger::console(), Desktop::default());
    assert!(type_line(&mut console, &mut desktop, "!ps -l").is_none());
    assert_eq!(
        console.pending.take().as_deref(),
        Some("ps -l"),
        "the bang line never reached the shell"
    );
    assert!(
        console.pending.take().is_none(),
        "the same command was handed over twice"
    );
}

/// An ordinary debugger command must not be mistaken for a shell command.
#[test]
fn a_plain_command_is_not_passed_to_the_shell() {
    let (mut console, mut desktop) = (debugger::console(), Desktop::default());
    type_line(&mut console, &mut desktop, "regs 2");
    assert!(console.pending.take().is_none());
}

/// The pane says what it can do without being asked.
///
/// It used to open with "type help", which keeps every command behind a word
/// you have to already know.
#[test]
fn the_pane_opens_with_its_commands_listed() {
    let mut desktop = Desktop::default();
    debugger::greet(&mut desktop);
    let text = screen(&mut desktop);
    assert!(text.contains("regs"), "opened without its commands: {text}");
    assert!(
        text.contains("!ps -l"),
        "opened without the shell escape: {text}"
    );
    assert!(
        !text.contains("type help"),
        "still hiding its commands behind a word: {text}"
    );
}

/// A rewind is exactly when the help is wanted, because the screen has just
/// been cleared of whatever went wrong.
#[test]
fn a_rewind_the_target_announces_reprints_the_help() {
    use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
    use swtos_session::routing;
    use swtos_session::state::Panes;

    let mut panes = Panes {
        desktop: Desktop::default(),
        resources: Default::default(),
    };
    let mut console = debugger::console();

    // Not greeted: this must come from the banner alone.
    let banner = |text: &str| {
        StreamItem::Frame(Frame {
            kind: FrameType::TtyOutput,
            channel: routing::SHELL,
            payload: text.as_bytes().to_vec(),
        })
    };
    routing::route(&mut panes, &mut console, 0.0, banner("SHELL RESTARTED\n"));
    assert!(
        screen(&mut panes.desktop).contains("regs"),
        "a restart left the pane without its commands"
    );

    let mut panes = Panes {
        desktop: Desktop::default(),
        resources: Default::default(),
    };
    routing::route(&mut panes, &mut console, 0.0, banner("SYSTEM REBOOTED\n"));
    assert!(
        screen(&mut panes.desktop).contains("regs"),
        "a reboot left the pane without its commands"
    );

    // Ordinary shell output must not trigger it, or the pane fills with help.
    let mut panes = Panes {
        desktop: Desktop::default(),
        resources: Default::default(),
    };
    routing::route(&mut panes, &mut console, 0.0, banner("READY\n"));
    assert!(
        !screen(&mut panes.desktop).contains("regs"),
        "ordinary output was mistaken for a rewind"
    );
}
