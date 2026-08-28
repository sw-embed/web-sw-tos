//! One live SWTOS session: the emulator, the virtual UART, and the bytes the
//! target has produced so far.
//!
//! Before the framed transport is negotiated SWTOS speaks plain bytes, so the
//! boot banner and shell menu arrive as ordinary text. That is what this
//! renders. Framing, panes, and channels arrive with the display modules.

use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{HEARTBEAT_BYTE_CYCLES, VirtualUart};

/// Cycles to run per tick beyond the heartbeat's own pacing. Matches the
/// batch size the command-line adapter uses.
const BATCH: u64 = 50_000;

/// Retained output, in bytes. The pane model brings real scrollback later.
const RETAIN: usize = 8192;

#[derive(Default)]
pub struct Session {
    pump: Pump,
    uart: VirtualUart,
    tick: u32,
    output: Vec<u8>,
}

impl Session {
    /// Advance `steps` 100 Hz scheduler ticks.
    pub fn step_many(&mut self, steps: u32) {
        for _ in 0..steps {
            self.uart
                .send(&heartbeat_frame(self.tick), HEARTBEAT_BYTE_CYCLES);
            self.tick = self.tick.wrapping_add(1);
            self.pump.run(&mut self.uart, BATCH);
            self.output.extend(self.uart.receive());
        }
        if self.output.len() > RETAIN {
            self.output.drain(..self.output.len() - RETAIN);
        }
    }

    /// Everything the target has printed, as text. Bytes outside printable
    /// ASCII are shown as `.` so a stray control byte cannot corrupt the grid.
    pub fn text(&self) -> String {
        self.output
            .iter()
            .map(|byte| match byte {
                b'\n' | b'\r' => *byte as char,
                0x20..=0x7e => *byte as char,
                _ => '.',
            })
            .collect()
    }

    /// Ticks elapsed, and entries held by the emulator's UART log. That log
    /// grows without bound and has no public clear, so it is worth watching.
    pub fn stats(&self) -> (u32, usize) {
        (self.tick, self.pump.log_len())
    }
}
