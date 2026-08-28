//! One live SWTOS session: the emulator, the virtual UART, the transport
//! decoder, and the pane desktop the target's output is painted into.
//!
//! Before the framed transport is negotiated SWTOS speaks plain bytes, which
//! the decoder surfaces as `StreamItem::Plain` and which belong to channel 0,
//! the Shell pane. Channels, applications, and the debugger arrive with
//! framing; the desktop is already shaped for them.

use swtos_frontend::protocol::{ConnectionDecoder, StreamItem};
use swtos_frontend::ui::{Cell, Desktop};
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{HEARTBEAT_BYTE_CYCLES, VirtualUart};

/// Cycles to run per tick beyond the heartbeat's own pacing. Matches the
/// batch size the command-line adapter uses.
const BATCH: u64 = 50_000;

/// Cycles to run after each typed byte. The command-line adapter spends
/// `FRAME_BYTE_CYCLES` (500,000) per byte, but that budget exists to let the
/// target consume a whole frame; at ~150,000 cycles per tick it would stall
/// the page for tens of milliseconds per keystroke. A keystroke only has to
/// be lifted out of the UART receive ring, so it gets the heartbeat's smaller
/// budget. Verified by typing at the live shell, not by reasoning alone.
const KEY_BYTE_CYCLES: u64 = HEARTBEAT_BYTE_CYCLES;

/// Channel zero is the Shell pane, and is where unframed output belongs.
const SHELL: u8 = 0;

#[derive(Default)]
pub struct Session {
    pump: Pump,
    uart: VirtualUart,
    decoder: ConnectionDecoder,
    desktop: Desktop,
    tick: u32,
}

impl Session {
    /// Advance at most `steps` 100 Hz scheduler ticks, stopping early once
    /// `deadline` (a `Date::now()`-style millisecond stamp) passes, and route
    /// whatever the target emits through the transport decoder into the
    /// desktop. Returns the average milliseconds spent per tick.
    ///
    /// Bounded by time rather than by count so the browser gets the thread
    /// back on a predictable schedule regardless of what the target is doing.
    pub fn run_until(&mut self, steps: u32, deadline: f64) -> f64 {
        let started = js_sys::Date::now();
        let mut done = 0;
        while done < steps {
            self.uart
                .send(&heartbeat_frame(self.tick), HEARTBEAT_BYTE_CYCLES);
            self.tick = self.tick.wrapping_add(1);
            self.pump.run(&mut self.uart, BATCH);
            let output = self.uart.receive();
            for item in self.decoder.push(&output) {
                if let StreamItem::Plain(bytes) = item {
                    self.desktop.push_channel(SHELL, &bytes);
                }
            }
            done += 1;
            if js_sys::Date::now() >= deadline {
                break;
            }
        }
        (js_sys::Date::now() - started) / f64::from(done.max(1))
    }

    /// The whole screen, as exactly `rows` rows of `cols` cells.
    pub fn grid(&self, cols: usize, rows: usize) -> Vec<Vec<Cell>> {
        self.desktop.render_grid(cols, rows)
    }

    /// Ticks elapsed, and entries held by the emulator's UART log. That log
    /// grows without bound and has no public clear, so it is worth watching.
    pub fn stats(&self) -> (u32, usize) {
        (self.tick, self.pump.log_len())
    }

    /// Translate a browser key event into the bytes a terminal would send,
    /// echo them into the Shell pane, and queue them for the target.
    ///
    /// SWTOS speaks plain bytes until the framed transport is negotiated, so
    /// the shell reads these directly. Ctrl-A through Ctrl-Z collapse to
    /// 0x01..0x1a exactly as a real terminal encodes them, which matters
    /// because Ctrl-A becomes the frontend prefix once panes exist. Named
    /// keys such as "Shift" or "ArrowUp" are longer than one character and
    /// are dropped rather than typed into the shell as words.
    ///
    /// The echo is not cosmetic: SWTOS never echoes input, so without it
    /// typing looks broken while every byte is in fact arriving. Enter echoes
    /// as a newline rather than the carriage return sent to the target,
    /// because a pane treats CR as "restart this line". Returns what was
    /// queued, which is how the mapping is tested.
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
        let echo: Vec<u8> = bytes
            .iter()
            .map(|byte| if *byte == b'\r' { b'\n' } else { *byte })
            .collect();
        self.desktop.push_channel(SHELL, &echo);
        self.uart.send(&bytes, KEY_BYTE_CYCLES);
        bytes
    }
}
