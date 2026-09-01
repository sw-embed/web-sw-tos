//! The footer clock belongs to the frontend, not to the target.
//!
//! It used to be updated only on the framed path, so it stopped whenever the
//! target did. That made a wedged session look exactly like a quiet one at a
//! glance, and it was reported as "the footer clock is not incrementing" when
//! the real fault was upstream of it.

mod fake;

use fake::FakeClock;
use swtos_frontend::protocol::Mode;
use swtos_session::state::Session;
use swtos_session::{driver, sending};

fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(120, 30)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn the_footer_clock_runs_before_the_target_has_answered() {
    // A fresh session has negotiated nothing, which is the state this is
    // about. Driving `periodic` directly keeps it that way: running the
    // emulator would negotiate within a tick or two and test the framed path
    // by accident.
    let mut session = driver::session();
    assert_eq!(session.transport.decoder.mode(), Mode::Plain);

    sending::periodic(&mut session, &FakeClock::new(0.0));

    let text = screen(&session);
    assert!(
        text.contains("13:45:07"),
        "the clock waited for the target: {text}"
    );
    assert!(
        !text.contains("--:--:--"),
        "the clock never started: {text}"
    );
    assert_eq!(
        session.transport.decoder.mode(),
        Mode::Plain,
        "the clock update must not have negotiated anything"
    );
}
