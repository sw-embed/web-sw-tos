//! Negotiation and the periodic traffic that follows it.
//!
//! These paths were previously unreachable from a test: they read the clock
//! directly, so they only ran in a browser. Both defects this crate exists to
//! prevent -- a dropped HELLO guard and a swallowed prefix key -- shipped
//! because of that. With the clock injected they are ordinary tests.
//!
//! Observed through effects rather than by sniffing the wire: the pump
//! consumes queued bytes as it runs, so draining the queue afterwards sees
//! nothing and an assertion on it passes vacuously. That mistake was made
//! here once already.

mod fake;

use fake::FakeClock;
use swtos_frontend::protocol::Mode;
use swtos_session::state::Session;
use swtos_session::{build, driver};

fn framed() -> Session {
    let mut session = build::session();
    driver::run(&mut session, 600, f64::MAX, &FakeClock::stopped());
    assert_eq!(
        session.transport.decoder.mode(),
        Mode::Framed,
        "the target never acknowledged HELLO"
    );
    session
}

fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(120, 43)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The first HELLO goes out immediately and schedules a retry.
///
/// The retry itself is a safety net that rarely fires: this target answers
/// within about five ticks, well inside the 25-tick interval. So the
/// observable fact is that an offer happened and armed the next one -- not
/// that a second offer was seen, which against a healthy target it never is.
#[test]
fn the_first_hello_goes_out_and_arms_a_retry() {
    let mut session = build::session();
    assert_eq!(session.transport.next_hello, 0, "nothing offered yet");
    driver::run(&mut session, 1, f64::MAX, &FakeClock::stopped());
    assert!(
        session.transport.next_hello > 0,
        "no HELLO was offered on the first tick, so a target that missed \
         early boot would never be reached"
    );
}

/// Once framed the retry must stop.
///
/// This is the regression that produced the endless menu: the target accepts
/// a repeat HELLO while framed and treats it as a fresh attach, re-running
/// catalog autostart and reprinting the menu on every retry.
#[test]
fn hello_stops_once_the_transport_is_framed() {
    let mut session = framed();
    let deadline = session.transport.next_hello;
    let before = screen(&session).matches("MENU").count();

    driver::run(&mut session, 400, f64::MAX, &FakeClock::stopped());

    assert_eq!(
        session.transport.next_hello, deadline,
        "the HELLO retry kept firing after negotiation"
    );
    let after = screen(&session).matches("MENU").count();
    assert!(
        after <= before + 1,
        "the menu was reprinted {} times, which is what a repeat HELLO does",
        after - before
    );
}

/// Once framed the session feeds the clock consumers and asks for resources.
/// Both are visible on screen: the status line carries the clock, and the
/// monitor pane fills only when a snapshot is requested and assembled.
#[test]
fn periodic_traffic_reaches_the_status_line_and_the_monitor() {
    let mut session = framed();
    driver::run(&mut session, 600, f64::MAX, &FakeClock::stopped());
    let text = screen(&session);

    assert!(
        text.contains("13:45:07"),
        "the status clock was never set from the injected clock: {text}"
    );
    assert!(
        text.contains("slots="),
        "the monitor pane was never asked for data: {text}"
    );
}

/// A deadline stops the run early, so the browser gets its thread back.
#[test]
fn the_deadline_bounds_the_work() {
    let mut session = build::session();
    let ran = driver::run(&mut session, 500, 50.0, &FakeClock::ticking(10.0));
    assert!(ran < 500, "the deadline was ignored: {ran} ticks ran");
}
