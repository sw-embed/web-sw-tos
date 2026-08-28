//! The Debugger pane is a local console, not a TTY.
//!
//! Its channel (254) has no terminal on the target, so anything sent there as
//! TTY_INPUT is silently discarded -- the exact symptom of "typing help in the
//! debugger does nothing". These pin that the console handles the keys itself.

use swtos_frontend::ui::Desktop;
use web_sw_tos::debugger::Console;
use web_sw_tos::session::Session;

fn type_line(console: &mut Console, desktop: &mut Desktop, line: &str) -> Option<Vec<u8>> {
    for ch in line.chars() {
        console.key(desktop, &ch.to_string());
    }
    console.key(desktop, "Enter")
}

/// Render the Debugger pane zoomed to the full screen. Unzoomed it is half
/// the width and the help line is clipped, which is a rendering artefact
/// rather than anything about the console.
fn screen(desktop: &mut Desktop) -> String {
    desktop.command(b'3');
    desktop.command(b'z');
    let text = render(desktop);
    desktop.command(b'z');
    text
}

fn render(desktop: &Desktop) -> String {
    desktop
        .render_grid(120, 43)
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn help_prints_the_command_list_into_the_pane() {
    let (mut console, mut desktop) = (Console::default(), Desktop::default());
    assert!(type_line(&mut console, &mut desktop, "help").is_none());
    let text = screen(&mut desktop);
    assert!(text.contains("regs"), "help did not list regs: {text}");
    assert!(text.contains("kill"), "help did not list kill");
}

/// `regs 2` must produce a DEBUG_REQUEST for the target, not screen text.
#[test]
fn regs_produces_a_request_for_the_target() {
    let (mut console, mut desktop) = (Console::default(), Desktop::default());
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
    let (mut console, mut desktop) = (Console::default(), Desktop::default());
    type_line(&mut console, &mut desktop, "dis 1000");
    let text = screen(&mut desktop);
    assert!(
        text.to_lowercase().contains("map"),
        "dis gave no hint that the debug map is absent: {text}"
    );
}

/// Keys typed at the Debugger pane must never reach the target.
#[test]
fn debugger_keys_do_not_go_out_as_tty_input() {
    let mut session = Session::default();
    session.send_key("a", true);
    session.send_key("3", false); // focus pane 3 = Debugger
    for key in ["h", "e", "l", "p", "Enter"] {
        assert!(
            session.send_key(key, false).is_empty(),
            "{key} leaked to the target from the Debugger pane"
        );
    }
}
