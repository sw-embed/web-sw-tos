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

/// Cycles to run after each typed byte. The command-line adapter spends
/// `FRAME_BYTE_CYCLES` (500,000) per byte, but that budget exists to let the
/// target consume a whole frame; at ~150,000 cycles per tick it would stall
/// the page for tens of milliseconds per keystroke. A keystroke only has to
/// be lifted out of the UART receive ring, so it gets the heartbeat's smaller
/// budget. Verified by typing at the live shell, not by reasoning alone.
const KEY_BYTE_CYCLES: u64 = HEARTBEAT_BYTE_CYCLES;

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

    /// Translate a browser key event into the bytes a terminal would send,
    /// echo them locally, and queue them for the target.
    ///
    /// SWTOS speaks plain bytes until the framed transport is negotiated, so
    /// the shell reads these directly. Ctrl-A through Ctrl-Z collapse to
    /// 0x01..0x1a exactly as a real terminal encodes them, which matters
    /// because Ctrl-A becomes the frontend prefix once panes exist. Named
    /// keys such as "Shift" or "ArrowUp" are longer than one character and
    /// are dropped rather than typed into the shell as words.
    ///
    /// The echo is not cosmetic: SWTOS never echoes input, so without it
    /// typing looks broken while every byte is in fact arriving. Returns what
    /// was queued, which is how the mapping is tested.
    pub fn send_key(&mut self, key: &str, ctrl: bool) -> Vec<u8> {
        let bytes: Vec<u8> = match (key, ctrl) {
            ("Enter", _) => vec![b'\r'],
            ("Backspace", _) => vec![0x08],
            ("Tab", _) => vec![b'\t'],
            ("Escape", _) => vec![0x1b],
            (name, true) if name.len() == 1 => match name.as_bytes()[0].to_ascii_uppercase() {
                c @ b'A'..=b'Z' => vec![c - b'A' + 1],
                _ => return Vec::new(),
            },
            (text, false) if text.chars().count() == 1 => text.as_bytes().to_vec(),
            _ => return Vec::new(),
        };
        for byte in &bytes {
            match byte {
                b'\r' => self.output.push(b'\n'),
                0x08 => _ = self.output.pop(),
                0x20..=0x7e => self.output.push(*byte),
                _ => {}
            }
        }
        self.uart.send(&bytes, KEY_BYTE_CYCLES);
        bytes
    }
}
