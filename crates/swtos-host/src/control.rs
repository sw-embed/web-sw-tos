//! Unframed control escapes the host puts on the wire.
//!
//! These bypass the framed transport entirely. They are read by the target's
//! UART interrupt handler, which sees every byte on the wire whatever it
//! belongs to, rather than by anything downstream of it. That is the whole
//! point of both of them: each exists for a moment when the code that would
//! normally read a frame is not running or not reading.
//!
//! The command-line frontend has to wrap these in a passthrough frame once its
//! link is framed, because `cor24-debug-adapter` sits in between reading frames
//! and discards whatever is not one. Here there is no adapter: the pump hands
//! bytes straight to the modeled UART, so they go out raw in every mode.

/// The scheduler heartbeat SWTOS needs for preemption, as the host sends it:
/// a fixed five-byte frame carrying a 24-bit little-endian tick. Sent at
/// 100 Hz. Without it SWTOS falls back to cooperative scheduling and a
/// process that never yields can never be interrupted.
pub fn heartbeat_frame(tick: u32) -> [u8; 5] {
    [0xff, 1, tick as u8, (tick >> 8) as u8, (tick >> 16) as u8]
}

/// Ask the target to restart its shell.
///
/// The shell is the one process that cannot be killed, because the session
/// ends with it, and it is not one of the leaves the preemption runway can
/// force to quiesce. A command running in the shell's own context therefore
/// owns the CPU until it chooses to give it back, and one that never does
/// takes the session with it.
///
/// Asking is the hard part, because a shell that needs this is one that has
/// stopped reading input: nothing drains the interrupt handler's ring, so
/// nothing downstream of it can ever see the request. The handler recognises
/// these two bytes itself and raises a flag the kernel acts on at its next
/// entry from the shell.
pub fn restart_request() -> [u8; 2] {
    [0xff, 4]
}

/// Ask the target for a warm reboot.
///
/// Where a shell restart rewinds endpoint 1 and keeps everything else, this
/// tears the system down and builds it again: child processes, the preemption
/// sidecar, the TTYs and the allocator. The kernel defers all of that to a
/// safe shell boundary before rewinding, so the request is the same shape as
/// a restart -- raised by the interrupt handler, acted on by the kernel when
/// it is next somewhere it can be.
pub fn reboot_request() -> [u8; 2] {
    [0xff, 5]
}
