//! Asking the target to recover.
//!
//! These are the ways out of a command that owns the CPU and will not give it
//! back. They have to work when nothing on the target is reading input, which
//! is why each request is two raw bytes read by the UART interrupt handler
//! rather than anything the framed transport carries.

use swtos_host::control::{reboot_request, restart_request};
use swtos_input::{dispatch, recovery};
use swtos_session::driver;
use swtos_session::state::Session;

/// Everything queued for the target since the session was made.
fn wire(session: &mut Session) -> Vec<u8> {
    let mut bytes = Vec::new();
    while let Some((byte, _)) = session.transport.uart.next_for_target() {
        bytes.push(byte);
    }
    bytes
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|slice| slice == needle)
}

/// Press Ctrl-A then one key.
fn prefix(session: &mut Session, key: &str) {
    dispatch::key(session, "a", true);
    dispatch::key(session, key, false);
}

#[test]
fn the_two_requests_are_distinct_and_unframed() {
    let mut session = driver::session();
    prefix(&mut session, "k");
    assert_eq!(wire(&mut session), restart_request());

    prefix(&mut session, "B");
    let sent = wire(&mut session);
    assert_eq!(sent, reboot_request());

    // A reboot is not a restart: they defer different amounts of cleanup, and
    // sending one where the other was asked for would silently do less.
    assert_ne!(reboot_request(), restart_request());
    // Unframed on purpose. A frame would be read by the framed transport,
    // which is downstream of the handler that has to see this.
    assert!(
        !contains(&sent, &[0xa5, 0x5a]),
        "wrapped in a frame: {sent:?}"
    );
}

#[test]
fn the_spellings_that_mean_restart_the_shell() {
    // The shell is endpoint 1, so killing it is the spelling a person already
    // knows from the shell's own kill command.
    assert!(recovery::is_shell_restart("kill 1"));
    assert!(recovery::is_shell_restart("kill ep=1"));
    assert!(recovery::is_shell_restart("  kill   1  "));

    // Killing anything else is an ordinary shell command and must be injected.
    assert!(!recovery::is_shell_restart("kill 3"));
    assert!(!recovery::is_shell_restart("kill ep=12"));
    assert!(!recovery::is_shell_restart("kill"));
    assert!(!recovery::is_shell_restart("kill 1 please"));
    assert!(!recovery::is_shell_restart("ps -l"));
}

#[test]
fn kill_1_at_the_debugger_restarts_rather_than_injecting() {
    let mut session = driver::session();
    prefix(&mut session, "3");
    for ch in "!kill 1".chars() {
        dispatch::key(&mut session, &ch.to_string(), false);
    }
    dispatch::key(&mut session, "Enter", false);

    let sent = wire(&mut session);
    assert!(
        contains(&sent, &restart_request()),
        "the request never reached the wire: {sent:?}"
    );
    // Injecting it would need the shell to be reading, which is the very
    // thing in doubt whenever this is asked for.
    assert!(
        !contains(&sent, b"kill 1"),
        "the command was injected as text instead: {sent:?}"
    );
}

#[test]
fn an_ordinary_kill_is_still_injected() {
    let mut session = driver::session();
    prefix(&mut session, "3");
    for ch in "!kill 3".chars() {
        dispatch::key(&mut session, &ch.to_string(), false);
    }
    dispatch::key(&mut session, "Enter", false);

    let sent = wire(&mut session);
    assert!(
        contains(&sent, b"kill 3"),
        "killing another endpoint must still reach the shell: {sent:?}"
    );
    assert!(
        !contains(&sent, &restart_request()),
        "killing another endpoint must not restart the shell: {sent:?}"
    );
}

/// Lowercase `b` is upstream's broadcast prefix, uppercase `B` is the reboot.
/// Confusing the two would send a reboot when a broadcast was meant.
#[test]
fn lowercase_b_is_not_a_reboot() {
    let mut session = driver::session();
    prefix(&mut session, "b");
    let sent = wire(&mut session);
    assert!(
        !contains(&sent, &reboot_request()),
        "broadcast was treated as a reboot: {sent:?}"
    );
}

/// The bytes are only half the claim: they have to reach the target and be
/// acted on. A warm reboot rebuilds the system, so the shell announces itself
/// again, which is evidence no amount of checking the wire would give.
#[test]
fn a_warm_reboot_brings_the_shell_back() {
    use swtos_frontend::protocol::Mode;
    use swtos_frontend::resource::Millis;
    use swtos_session::state::{Clock, LocalTime};

    struct Stopped;
    impl Clock for Stopped {
        fn elapsed(&self) -> Millis {
            0.0
        }
        fn local(&self) -> LocalTime {
            LocalTime::default()
        }
    }
    let settle = |session: &mut Session| {
        driver::run(session, 700, f64::MAX, &Stopped);
    };
    let screen = |session: &Session| -> String {
        session
            .panes
            .desktop
            .render_grid(160, 44)
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);
    let before = screen(&session).matches("MENU").count();

    prefix(&mut session, "B");
    settle(&mut session);
    settle(&mut session);

    let after = screen(&session);
    assert!(
        after.matches("MENU").count() > before,
        "the reboot request never reached the target: {after}"
    );
}
