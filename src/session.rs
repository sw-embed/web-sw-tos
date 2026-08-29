//! One live SWTOS session: the emulator, the virtual UART, the transport
//! decoder, and the pane desktop the target's output is painted into.
//!
//! Before the framed transport is negotiated SWTOS speaks plain bytes, which
//! the decoder surfaces as `StreamItem::Plain` and which belong to channel 0,
//! the Shell pane. Channels, applications, and the debugger arrive with
//! framing; the desktop is already shaped for them.
//!
//! Three things about key handling are worth stating once, here, rather than
//! at each branch of `send_key`:
//!
//! - Typed input is echoed locally because SWTOS never echoes. Without it
//!   typing looks broken while every byte is in fact arriving. Control bytes
//!   are filtered from the echo: a raw Escape renders as U+FFFD in a pane.
//! - Prefix-`e` is the one frontend command that reaches the target. It sends
//!   a real Escape, which is how Uptime and Clock are stopped, and cannot be
//!   typed directly once copy mode and the help overlay also want Escape.
//! - The Debugger and Resources panes are local consoles, not terminals.
//!   Their channels have no TTY on the target, so routing their keys out as
//!   TTY_INPUT discards every keystroke silently.

use crate::debugger::Console;
use crate::keys;
use crate::transport;
use swtos_frontend::protocol::{ConnectionDecoder, Mode};
use swtos_frontend::ui::{Cell, Desktop};
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{HEARTBEAT_BYTE_CYCLES, VirtualUart};

/// Cycles to run per tick beyond the heartbeat's own pacing. Matches the
/// batch size the command-line adapter uses.
const BATCH: u64 = 50_000;

/// What the status line needs, gathered in one call so the session does not
/// grow an accessor per field.
pub struct Status {
    pub tick: u32,
    pub log_entries: usize,
    pub framed: bool,
    pub prefix_armed: bool,
}

/// Ticks between time frames. The consumers display centiseconds as seconds,
/// so four a second is plenty and keeps frame traffic off the critical path.
const TIME_TICK_INTERVAL: u32 = 25;

/// Ticks between HELLO attempts while still in plain mode. The target is not
/// listening during early boot, so a single HELLO at startup is not enough;
/// the CLI retries every 250 ms and this is the same cadence at 100 Hz.
const HELLO_RETRY_TICKS: u32 = 25;

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
    /// Tick at which the next HELLO goes out, while still unnegotiated.
    next_hello: u32,
    /// The Debugger pane's local console. Not a TTY; see `debugger`.
    console: Console,
    /// Set once the debugger has greeted, after the transport goes framed.
    greeted: bool,
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
            self.negotiate();
            self.uart
                .send(&heartbeat_frame(self.tick), HEARTBEAT_BYTE_CYCLES);
            self.tick = self.tick.wrapping_add(1);
            self.pump.run(&mut self.uart, BATCH);
            let output = self.uart.receive();
            for item in self.decoder.push(&output) {
                transport::route(&mut self.desktop, &mut self.console, item);
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

    /// Install the runtime-fetched debug map.
    pub fn load_map(&mut self, json: &str) {
        self.console.load_map(&mut self.desktop, json);
    }

    /// Everything the status line reports. `log_entries` is the emulator's
    /// UART log, which grows without bound and has no public clear.
    pub fn status(&self) -> Status {
        Status {
            tick: self.tick,
            log_entries: self.pump.log_len(),
            framed: self.decoder.mode() == Mode::Framed,
            prefix_armed: self.prefix_armed,
        }
    }

    /// Handle one key. Returns whatever was queued for the target, which is
    /// empty when the key was consumed by the frontend.
    ///
    /// Three consumers sit in front of the target, in order. A pending prefix
    /// makes this key a frontend command. Ctrl-A arms that prefix. Copy mode
    /// claims navigation keys. Only what none of them wants reaches SWTOS.
    ///
    pub fn send_key(&mut self, key: &str, ctrl: bool) -> Vec<u8> {
        if std::mem::take(&mut self.prefix_armed) {
            return self.prefix_command(key);
        }
        if ctrl && key.eq_ignore_ascii_case(PREFIX) {
            self.prefix_armed = true;
            return Vec::new();
        }
        if self.desktop.copy_mode_enabled() && keys::copy_motion(&mut self.desktop, key) {
            return Vec::new();
        }
        let kind = self.desktop.focused_kind();
        if self
            .console
            .consume(kind, &mut self.desktop, &mut self.uart, key)
        {
            return Vec::new();
        }
        let bytes = keys::to_bytes(key, ctrl);
        let channel = self.desktop.focused_channel();
        self.desktop
            .push_channel(channel, &keys::echo_bytes(&bytes));
        transport::transmit(&mut self.uart, &self.decoder, channel, &bytes);
        bytes
    }

    /// Run one frontend command. Prefix-`e` is the exception that reaches the
    /// target: it is how a running application is sent a real Escape.
    fn prefix_command(&mut self, key: &str) -> Vec<u8> {
        // `c` is ours: upstream has no clear command, and intercepting it here
        // keeps the vendored command table untouched.
        if key == "c" {
            self.desktop.clear_focused();
            return Vec::new();
        }
        if key == "e" {
            let channel = self.desktop.focused_channel();
            transport::transmit(&mut self.uart, &self.decoder, channel, &[0x1b]);
            return vec![0x1b];
        }
        self.desktop.command(keys::command_byte(key));
        Vec::new()
    }

    /// Offer HELLO until framed, then greet in the Debugger pane once, and
    /// keep the clock consumers fed.
    fn negotiate(&mut self) {
        if self.decoder.mode() == Mode::Framed && self.tick.is_multiple_of(TIME_TICK_INTERVAL) {
            transport::time_ticks(&mut self.uart, &mut self.desktop, self.tick);
        }
        if self.tick >= self.next_hello {
            transport::negotiate(&mut self.uart, &self.decoder);
            self.next_hello = self.tick.wrapping_add(HELLO_RETRY_TICKS);
        }
        if self.decoder.mode() == Mode::Framed && !self.greeted {
            self.greeted = true;
            self.desktop.set_error(None);
            let request = self.console.greet(&mut self.desktop);
            transport::request(&mut self.uart, request);
        }
    }
}
