//! Asking the target to recover: restart its shell, or reboot outright.
//!
//! Both requests are two raw bytes read by the target's UART interrupt
//! handler, because the session that needs either has usually stopped reading
//! anything: nothing drains the handler's ring, so nothing downstream of it
//! can ever see the request. The handler is the only code still running.
//!
//! Three ways in: `Ctrl-A k` restarts the shell, `Ctrl-A B` reboots, and
//! `!kill 1` typed at the Debugger pane restarts. That last one is not
//! injected as a shell command the way `!ps -l` is -- injection needs the
//! shell to be reading, which is precisely what is in doubt.

use swtos_host::control::{reboot_request, restart_request};
use swtos_host::uart::HEARTBEAT_BYTE_CYCLES;
use swtos_session::debugger;
use swtos_session::state::Session;

/// Does this Debugger-pane command mean "restart the shell"?
///
/// The shell is endpoint 1, so a request to kill it is the spelling a person
/// already knows, in the forms the shell itself accepts.
pub fn is_shell_restart(command: &str) -> bool {
    let mut words = command.split_whitespace();
    if words.next() != Some("kill") {
        return false;
    }
    let target = words.next().map(|word| word.trim_start_matches("ep="));
    words.next().is_none() && target == Some("1")
}

/// Rewind the shell to its entry point, keeping everything else.
pub fn restart_shell(session: &mut Session) {
    request(session, &restart_request(), b"restarting the shell\n");
}

/// Tear the system down and build it again.
pub fn reboot_system(session: &mut Session) {
    request(
        session,
        &reboot_request(),
        b"requesting warm SWTOS reboot\n",
    );
}

/// Put a recovery request on the wire and say so in the Debugger pane.
///
/// Raw, not framed, and paced like a heartbeat. The command-line frontend has
/// to wrap these once its link is framed, because `cor24-debug-adapter` reads
/// frames and discards whatever is not one; here the pump hands bytes straight
/// to the modeled UART, so there is nothing in the way.
fn request(session: &mut Session, bytes: &[u8], note: &[u8]) {
    session.transport.uart.send(bytes, HEARTBEAT_BYTE_CYCLES);
    session.panes.desktop.push_channel(debugger::CHANNEL, note);
}
