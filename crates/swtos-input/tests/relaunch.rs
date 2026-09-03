//! Can the shell launch, and then launch again?
//!
//! The reported symptom was that after running one program the menu stopped
//! answering. Two sw-tos fixes were needed. 43734d5 stopped the shell joining
//! on every child rather than the one it started -- the monitor is a child and
//! never exits -- and 5ce74ac stopped the exiting program handing the keyboard
//! to whichever of the first two slots was occupied, which by then was the
//! monitor, blocked on a clock tick rather than waiting for a person.
//!
//! The numbered menu is deliberately synchronous: one program at a time. So
//! the sequence a person performs is launch, end it from its own pane, return
//! to the Shell, launch again -- and all four steps are checked here.

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

/// Focus a pane by its prefix-number.
fn focus(session: &mut Session, number: &str) {
    dispatch::key(session, dispatch::PREFIX_KEY, true);
    dispatch::key(session, number, false);
}

#[test]
fn the_shell_launches_a_second_program_after_the_first() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);

    // 1 = Hello. It prints and then waits for a key.
    type_line(&mut session, "1");
    settle(&mut session);
    let launched = screen(&session);
    assert!(
        launched.contains("Press key"),
        "menu choice 1 launched nothing: {launched}"
    );

    // End it from its own pane, as a person would.
    focus(&mut session, "4");
    dispatch::key(&mut session, " ", false);
    settle(&mut session);
    settle(&mut session);
    let dismissed = screen(&session);
    assert!(
        dismissed.contains("READY"),
        "the program never released the prompt: {dismissed}"
    );
    assert!(
        dismissed.contains("(ended)"),
        "the pane never admitted its process had gone: {dismissed}"
    );

    // Back to the Shell and choose again. 2 = Counter, which prints C1..C5:
    // output no menu text contains, so it cannot be confused with the banner.
    focus(&mut session, "1");
    type_line(&mut session, "2");
    settle(&mut session);
    settle(&mut session);
    let after = screen(&session);
    assert!(
        after.contains("C1"),
        "the second launch produced nothing: {after}"
    );
}

/// KNOWN ISSUE, and not one this side can fix alone.
///
/// A program that starts and finishes between two resource snapshots is never
/// seen running by either of them, so nothing renames its pane. Its output
/// therefore lands in a pane still carrying the previous program's name and
/// still marked `(ended)`. Upstream already accepted this argument once --
/// `push_channel` sets `ran` because output is better evidence than a snapshot
/// -- but it does not yet clear `ended` on the same reasoning, and `ended` is
/// private to the vendored file.
///
/// Asserted as it stands so this fails when upstream applies that reasoning.
#[test]
fn a_reused_pane_still_wears_the_previous_program_name() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);

    type_line(&mut session, "1");
    settle(&mut session);
    focus(&mut session, "4");
    dispatch::key(&mut session, " ", false);
    settle(&mut session);
    settle(&mut session);
    focus(&mut session, "1");
    type_line(&mut session, "2");
    settle(&mut session);
    settle(&mut session);

    let after = screen(&session);
    assert!(after.contains("C1"), "counter never ran: {after}");
    assert!(
        after.contains("hello ep=3 (ended)"),
        "the reused pane is named correctly now -- delete this test and assert \
         the pane says `counter` instead: {after}"
    );
}
