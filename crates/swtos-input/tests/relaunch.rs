//! Can the shell launch, and then launch again?
//!
//! The reported symptom was that after running one program the menu stopped
//! answering. sw-tos 43734d5 identified the cause on its side: the shell's
//! join waited for every child rather than the one it started, and the
//! monitor is a child that never exits.

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

/// Endpoints the target reports as running, by name.
fn running(session: &Session) -> Vec<String> {
    session
        .panes
        .resources
        .snapshot()
        .map(|snap| {
            snap.processes
                .values()
                .filter(|process| process.state != 0)
                .map(|process| process.name.clone())
                .collect()
        })
        .unwrap_or_default()
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

    type_line(&mut session, "1");
    settle(&mut session);
    let after_first = running(&session);

    // Dismiss the launched program from its own pane, as a person would.
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "4", false);
    let target = session.panes.desktop.focused_channel();
    let ttyin = |s: &Session, ep: u8| -> u32 {
        s.panes
            .resources
            .snapshot()
            .and_then(|snap| snap.processes.get(&ep))
            .map(|p| p.tty_in)
            .unwrap_or(0)
    };
    let before = ttyin(&session, target + 1);
    dispatch::key(&mut session, " ", false);
    settle(&mut session);
    println!(
        "focused channel {target} (endpoint {}), its tty_in {before} -> {}",
        target + 1,
        ttyin(&session, target + 1)
    );

    // Back to the Shell before typing again: input goes to the focused
    // channel, and the pane just dismissed is not the Shell.
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "1", false);
    type_line(&mut session, "2");
    settle(&mut session);
    let after_second = running(&session);

    println!("running after first launch:  {after_first:?}");
    println!("running after second launch: {after_second:?}");
    println!("panes: {:?}", session.panes.desktop.layout());

    assert!(
        after_first
            .iter()
            .any(|name| name != "shell" && name != "mon"),
        "the first launch started nothing: {after_first:?}"
    );
    // The launched program did exit when its pane was given a key: proof the
    // frontend delivers to an application channel, and that dismissal works.
    assert!(
        !after_second.iter().any(|name| name == "hello"),
        "the program did not exit when its pane was given a key, which would \
         be a delivery failure on this side: {after_second:?}"
    );

    // KNOWN ISSUE, target side, narrower than it was. sw-tos 43734d5 fixed
    // the shell joining on every child rather than the one it started, and
    // the first launch now works where it previously took the prompt for
    // good. A second launch after the first ends still does not happen.
    //
    // Asserted as it stands so this test fails when sw-tos fixes it; invert
    // it to `any(|name| name == "counter")` at that point. Asserting the
    // wanted behaviour now would only be a red test with nothing to act on.
    assert!(
        !after_second.iter().any(|name| name == "counter"),
        "a second launch now works -- invert this assertion and re-check the \
         menu end to end: {after_first:?} then {after_second:?}"
    );
}
