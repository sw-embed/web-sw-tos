//! Asking the target to restart its shell.
//!
//! Two ways in, because the shell that needs this has usually stopped reading
//! anything: `Ctrl-A k`, and `!kill 1` typed at the Debugger pane. The second
//! is not injected as a shell command the way `!ps -l` is -- injection needs
//! the shell to be reading, which is precisely what is in doubt.

use swtos_host::control::restart_request;
use swtos_host::uart::HEARTBEAT_BYTE_CYCLES;
use swtos_session::debugger;
use swtos_session::state::Session;

/// Does this Debugger-pane command mean "restart the shell"?
///
/// The shell is endpoint 1, so a request to kill it is the spelling a person
/// already knows, in the forms the shell itself accepts.
pub fn is_request(command: &str) -> bool {
    let mut words = command.split_whitespace();
    if words.next() != Some("kill") {
        return false;
    }
    let target = words.next().map(|word| word.trim_start_matches("ep="));
    words.next().is_none() && target == Some("1")
}

/// Put the request on the wire and say so in the Debugger pane.
///
/// Raw, not framed, and paced like a heartbeat: it is read by the target's
/// UART interrupt handler rather than by the framed transport.
pub fn send(session: &mut Session) {
    session
        .transport
        .uart
        .send(&restart_request(), HEARTBEAT_BYTE_CYCLES);
    session
        .panes
        .desktop
        .push_channel(debugger::CHANNEL, b"restarting the shell\n");
}
