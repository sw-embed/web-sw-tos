//! The in-process virtual UART that replaces the pty.
//!
//! In the command-line demo two processes are joined by a pty: `te-rs` writes
//! frames in and reads frames out, while `cor24-debug-adapter` hands those
//! bytes to the modeled UART. In the browser both sides live in one WASM
//! module, so the pty collapses into the two byte queues here.
//!
//! Bytes toward the target carry a cycle budget because delivery pacing is
//! part of the contract, not an implementation detail: each SWTOS input ring
//! holds only 16 bytes, and a full ring drops the new byte and increments an
//! overflow counter. The pump runs the budget between bytes so the target's
//! UART interrupt handler has time to drain the ring.

use std::collections::VecDeque;

/// Cycles to run after handing the modeled UART an ordinary frame byte.
/// Mirrors `TARGET_BYTE_CYCLES` in the command-line adapter.
pub const FRAME_BYTE_CYCLES: u64 = 500_000;

/// Cycles to run after a scheduler heartbeat byte. Heartbeats arrive at 100 Hz
/// and the UART interrupt handler consumes one in far fewer cycles than a
/// frame; spending the full frame budget on them makes the pump fall behind
/// real time and starves outbound resource snapshots. Mirrors
/// `HEARTBEAT_BYTE_CYCLES` in the command-line adapter.
pub const HEARTBEAT_BYTE_CYCLES: u64 = 20_000;

/// Two byte queues standing in for the pty.
#[derive(Debug, Default)]
pub struct VirtualUart {
    to_target: VecDeque<(u8, u64)>,
    to_host: VecDeque<u8>,
}

impl VirtualUart {
    /// Queue bytes for the target, each to be followed by `cycles_per_byte`
    /// of execution. Use [`FRAME_BYTE_CYCLES`] for framed traffic and
    /// [`HEARTBEAT_BYTE_CYCLES`] for the scheduler heartbeat.
    pub fn send(&mut self, bytes: &[u8], cycles_per_byte: u64) {
        self.to_target
            .extend(bytes.iter().map(|byte| (*byte, cycles_per_byte)));
    }

    /// Target side: the next byte to hand the modeled UART, with the number of
    /// cycles to run afterwards.
    pub fn next_for_target(&mut self) -> Option<(u8, u64)> {
        self.to_target.pop_front()
    }

    /// Target side: record bytes the modeled UART emitted.
    pub fn emit(&mut self, bytes: &[u8]) {
        self.to_host.extend(bytes);
    }

    /// Frontend side: take everything the target has emitted since last call.
    pub fn receive(&mut self) -> Vec<u8> {
        self.to_host.drain(..).collect()
    }
}
