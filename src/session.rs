//! One live SWTOS session: the emulator, the virtual UART, the transport
//! decoder, and the pane desktop the target's output is painted into.
//!
//! Before the framed transport is negotiated SWTOS speaks plain bytes, which
//! the decoder surfaces as `StreamItem::Plain` and which belong to channel 0,
//! the Shell pane. Channels, applications, and the debugger arrive with
//! framing; the desktop is already shaped for them.

use crate::keys;
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

/// The frontend prefix, as a terminal sends it: Ctrl-A is 0x01.
const PREFIX: &str = "a";

#[derive(Default)]
pub struct Session {
    pump: Pump,
    uart: VirtualUart,
    decoder: ConnectionDecoder,
    desktop: Desktop,
    tick: u32,
    /// Set by the prefix key; the next key is a frontend command, not input.
    prefix_armed: bool,
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

    /// Handle one key. Returns whatever was queued for the target, which is
    /// empty when the key was consumed by the frontend.
    ///
    /// Three consumers sit in front of the target, in order. A pending prefix
    /// makes this key a frontend command. Ctrl-A arms that prefix. Copy mode
    /// claims navigation keys. Only what none of them wants reaches SWTOS.
    ///
    /// Typed input is echoed locally because SWTOS never echoes: without it
    /// typing looks broken while every byte is in fact arriving. Control bytes
    /// are filtered out of the echo -- pushing a raw Escape into a pane
    /// renders it as a replacement character.
    pub fn send_key(&mut self, key: &str, ctrl: bool) -> Vec<u8> {
        if std::mem::take(&mut self.prefix_armed) {
            // Prefix-e sends a real Escape to the focused application, which
            // is how Uptime and Clock are stopped. It cannot be typed
            // directly once copy mode and help also want Escape.
            if key == "e" {
                self.uart.send(&[0x1b], KEY_BYTE_CYCLES);
                return vec![0x1b];
            }
            self.desktop.command(keys::command_byte(key));
            return Vec::new();
        }
        if ctrl && key.eq_ignore_ascii_case(PREFIX) {
            self.prefix_armed = true;
            return Vec::new();
        }
        if self.desktop.copy_mode_enabled() && keys::copy_motion(&mut self.desktop, key) {
            return Vec::new();
        }
        let bytes = keys::to_bytes(key, ctrl);
        self.desktop.push_channel(SHELL, &keys::echo_bytes(&bytes));
        self.uart.send(&bytes, KEY_BYTE_CYCLES);
        bytes
    }

    /// True while the prefix is armed, for the status line.
    pub fn prefix_armed(&self) -> bool {
        self.prefix_armed
    }
}
