//! Can the shell launch, and then launch again?
//!
//! The reported symptom was that after running one program the menu stopped
//! answering. Two sw-tos fixes were needed. 43734d5 stopped the shell joining
//! on every child rather than the one it started -- the monitor is a child and
//! never exits -- and 5ce74ac stopped the exiting program handing the keyboard
//! to whichever of the first two slots was occupied, which by then was the
//! monitor, blocked on a clock tick rather than waiting for a person.
//!
//! sw-tos d86eade then moved the menu's programs into the shell itself, where
//! a foreground program has always belonged. There is no second pane to visit
//! and nothing in the process table to kill: one pane, one thing typing
//! reaches, and a key ends it. `bg` is what gives a program a pane of its own.
//! So the sequence a person performs is launch, end it where it is running,
//! launch again.

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
    driver::run(session, 700, f64::MAX, &Stopped);
}

/// The whole screen as text, which is the evidence a person actually has.
fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(160, 44)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn type_line(session: &mut Session, text: &str) {
    for ch in text.chars() {
        dispatch::key(session, &ch.to_string(), false);
    }
    dispatch::key(session, "Enter", false);
    settle(session);
}

#[test]
fn the_shell_launches_a_second_program_after_the_first() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);

    // 1 = Hello, which runs in the shell and says how to leave.
    type_line(&mut session, "1");
    settle(&mut session);
    let launched = screen(&session);
    assert!(
        launched.contains("Press a key here to exit"),
        "menu choice 1 launched nothing: {launched}"
    );

    // End it where it is running. No pane to visit, because it never got one.
    dispatch::key(&mut session, " ", false);
    settle(&mut session);
    settle(&mut session);
    let dismissed = screen(&session);
    assert!(
        dismissed.contains("MENU"),
        "the program never released the prompt: {dismissed}"
    );

    // Choose again. 2 = Counter, whose steps are output no menu text contains,
    // so they cannot be confused with the banner. The letter tracks the
    // endpoint the step ran on: it read C when the menu spawned a process of
    // its own, and reads A now that the program runs in the shell's own slot.
    type_line(&mut session, "2");
    settle(&mut session);
    settle(&mut session);
    let after = screen(&session);
    assert!(
        after.contains("A1"),
        "the second launch produced nothing: {after}"
    );
}

/// A program told to outlive the prompt still gets a pane, and still answers
/// `kill`. That is the half of the old behaviour worth keeping, and it is the
/// only route to a pane now.
#[test]
fn a_background_program_gets_a_pane_and_answers_kill() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);

    type_line(&mut session, "bg uptime");
    settle(&mut session);
    settle(&mut session);
    let running = screen(&session);
    assert!(running.contains("uptime"), "bg started nothing: {running}");

    // Its endpoint, from the snapshot rather than from guessing at slots.
    let endpoint = session
        .panes
        .resources
        .snapshot()
        .and_then(|snap| {
            snap.processes
                .values()
                .find(|process| process.name.starts_with("upti") && process.state != 0)
                .map(|process| process.endpoint)
        })
        .expect("uptime must be in the process table");

    type_line(&mut session, &format!("kill {endpoint}"));
    settle(&mut session);
    settle(&mut session);

    let alive = session
        .panes
        .resources
        .snapshot()
        .map(|snap| {
            snap.processes
                .values()
                .any(|p| p.name.starts_with("upti") && p.state != 0)
        })
        .unwrap_or(false);
    assert!(
        !alive,
        "kill {endpoint} was accepted and never serviced: {}",
        screen(&session)
    );
}
