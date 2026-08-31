//! The Debugger pane is a local console, not a TTY.
//!
//! Its channel (254) has no terminal on the target, so anything sent there as
//! TTY_INPUT is silently discarded -- the exact symptom of "typing help in the
//! debugger does nothing". These pin that the console handles the keys itself.

use swtos_frontend::ui::Desktop;
use swtos_session::state::Console;
use swtos_session::{build, debugger};

fn type_line(console: &mut Console, desktop: &mut Desktop, line: &str) -> Option<Vec<u8>> {
    for ch in line.chars() {
        debugger::key(console, desktop, &ch.to_string());
    }
    debugger::key(console, desktop, "Enter")
}

/// Render the Debugger pane zoomed to the full screen. Unzoomed it is half
/// the width and the help line is clipped, which is a rendering artefact
/// rather than anything about the console.
fn screen(desktop: &mut Desktop) -> String {
    desktop.command(b'3');
    desktop.command(b'z');
    // Rendered wider than any real screen: the help line runs past 150
    // characters, so a realistic width clips its tail and an assertion about
    // the far end would be testing the renderer, not the console.
    let text = render(desktop, 220, 50);
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
    let (mut console, mut desktop) = (build::console(), Desktop::default());
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
    let (mut console, mut desktop) = (build::console(), Desktop::default());
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
    let (mut console, mut desktop) = (build::console(), Desktop::default());
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
    let (mut console, mut desktop) = (build::console(), Desktop::default());
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
    let (mut console, mut desktop) = (build::console(), Desktop::default());
    type_line(&mut console, &mut desktop, "regs 2");
    assert!(console.pending.take().is_none());
}
