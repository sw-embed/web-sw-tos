//! Asking the target to restart its shell.
//!
//! This is the way out of a command that owns the CPU and will not give it
//! back. It has to work when nothing on the target is reading input, which is
//! why the request is two raw bytes read by the UART interrupt handler rather
//! than anything the framed transport carries.

use swtos_host::control::restart_request;
use swtos_input::{dispatch, restart};
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

#[test]
fn the_spellings_that_mean_restart_the_shell() {
    // The shell is endpoint 1, so killing it is the spelling a person already
    // knows from the shell's own kill command.
    assert!(restart::is_request("kill 1"));
    assert!(restart::is_request("kill ep=1"));
    assert!(restart::is_request("  kill   1  "));

    // Killing anything else is an ordinary shell command and must be injected.
    assert!(!restart::is_request("kill 3"));
    assert!(!restart::is_request("kill ep=12"));
    assert!(!restart::is_request("kill"));
    assert!(!restart::is_request("kill 1 please"));
    assert!(!restart::is_request("ps -l"));
}

#[test]
fn ctrl_a_k_puts_the_request_on_the_wire_unframed() {
    let mut session = driver::session();
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "k", false);

    let sent = wire(&mut session);
    assert_eq!(
        sent,
        restart_request(),
        "Ctrl-A k must send exactly the two-byte request: {sent:?}"
    );
    // Unframed on purpose. A frame would be read by the framed transport,
    // which is downstream of the handler that has to see this.
    assert!(
        !contains(&sent, &[0xa5, 0x5a]),
        "wrapped in a frame: {sent:?}"
    );
}

#[test]
fn kill_1_at_the_debugger_restarts_rather_than_injecting() {
    let mut session = driver::session();
    // Focus the Debugger pane, where `!` hands a line to the shell.
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "3", false);
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
    dispatch::key(&mut session, "a", true);
    dispatch::key(&mut session, "3", false);
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
