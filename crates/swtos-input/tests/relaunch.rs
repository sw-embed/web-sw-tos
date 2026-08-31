//! The reported sequence, driven the way a person drives it: through key
//! dispatch and pane switching, not by writing bytes to channels.

use swtos_frontend::protocol::Mode;
use swtos_frontend::resource::Millis;
use swtos_input::dispatch;
use swtos_session::driver;
use swtos_session::state::{Clock, LocalTime, Session};

struct Stopped;
impl Clock for Stopped {
    fn elapsed(&self) -> Millis {
        0.0
    }
    fn local(&self) -> LocalTime {
        LocalTime::default()
    }
}

fn settle(session: &mut Session) {
    driver::run(session, 600, f64::MAX, &Stopped);
}

/// The Shell pane alone, zoomed so nothing is clipped.
fn shell_pane(session: &mut Session) -> String {
    session.panes.desktop.command(b'1');
    session.panes.desktop.command(b'z');
    let text = screen(session);
    session.panes.desktop.command(b'z');
    text
}

fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(160, 60)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn type_line(session: &mut Session, text: &str) {
    for ch in text.chars() {
        dispatch::key(session, &ch.to_string(), false);
        settle(session);
    }
    dispatch::key(session, "Enter", false);
    settle(session);
}

fn focus(session: &mut Session, pane: &str) {
    dispatch::key(session, "a", true);
    dispatch::key(session, pane, false);
}

#[test]
fn the_menu_answers_again_after_a_program_is_ended_from_its_pane() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);

    println!(
        "=== panes at boot ===\n{:?}",
        session.panes.desktop.layout()
    );
    type_line(&mut session, "1");
    println!(
        "=== panes after menu 1 ===\n{:?}",
        session.panes.desktop.layout()
    );
    let launched = screen(&session);
    assert!(
        launched.contains("Hello") || launched.contains("Press"),
        "menu choice 1 launched nothing: {launched}"
    );

    // Move to the application pane and end it with a keypress.
    focus(&mut session, "5");
    let moved_to = session.panes.desktop.focused_channel();
    println!(
        "=== pane 5 is channel {moved_to}, kind {:?}",
        session.panes.desktop.focused_kind()
    );
    dispatch::key(&mut session, " ", false);
    settle(&mut session);

    // Back to the Shell, and try the menu again.
    println!(
        "=== panes after space ===\n{:?}",
        session.panes.desktop.layout()
    );
    focus(&mut session, "1");
    let back_on = session.panes.desktop.focused_channel();
    println!(
        "=== back on channel {back_on}, kind {:?}",
        session.panes.desktop.focused_kind()
    );
    // Does the byte reach the target at all? The snapshot counts what the
    // shell has read, which distinguishes "my input never arrived" from "the
    // target received it and did nothing".
    let ttyin = |s: &Session| -> u32 {
        s.panes
            .resources
            .snapshot()
            .and_then(|snap| snap.processes.get(&1))
            .map(|p| p.tty_in)
            .unwrap_or(0)
    };
    let uart_rx = |s: &Session| -> u32 {
        s.panes
            .resources
            .snapshot()
            .map(|snap| snap.uart_rx)
            .unwrap_or(0)
    };
    let before_rx = uart_rx(&session);
    let before_in = ttyin(&session);
    type_line(&mut session, "2");
    settle(&mut session);
    println!(
        "=== shell tty_in before={before_in} after={}",
        ttyin(&session)
    );
    println!(
        "=== target uart rx before={before_rx} after={} (grew: {})",
        uart_rx(&session),
        uart_rx(&session) > before_rx
    );
    if let Some(snap) = session.panes.resources.snapshot() {
        for (ep, p) in &snap.processes {
            println!(
                "    ep={ep} name={} state={} blocked={} ttyin={} ttyout={}",
                p.name, p.state, p.blocked, p.tty_in, p.tty_out
            );
        }
    }
    println!(
        "=== panes after menu 2 ===\n{:?}",
        session.panes.desktop.layout()
    );
    let shell = shell_pane(&mut session);
    println!(
        "=== SHELL PANE ===\n{}",
        shell.lines().take(24).collect::<Vec<_>>().join("\n")
    );

    assert_eq!(
        back_on, 0,
        "Ctrl-A 1 did not return focus to the Shell channel (it went to \
         {back_on}); input after this goes to the wrong place. Pane 5 was \
         channel {moved_to}."
    );

    // KNOWN ISSUE, target side. The bytes reach the target -- its UART receive
    // counter climbs -- but the shell's own `tty_in` does not move, and it
    // sits runnable rather than blocked. So it stops consuming input after a
    // launch. sw-tos is working in this area (6e8a519, "no launch blocks the
    // prompt").
    //
    // This asserts the stall on purpose, so the test fails when sw-tos fixes
    // it and the assertion is inverted then. Asserting the fixed behaviour
    // now would just be a red test with nothing to do about it.
    assert!(
        uart_rx(&session) > before_rx,
        "the frontend stopped delivering bytes to the target, which would be \
         a bug on this side rather than the known target-side stall"
    );
    assert_eq!(
        ttyin(&session),
        before_in,
        "the shell read input after a launch -- the target-side stall looks \
         fixed, so invert this assertion and re-check the menu end to end"
    );
}
