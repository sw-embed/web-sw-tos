//! One live SWTOS session: the emulator, the virtual UART, the transport
//! decoder, and the pane desktop the target's output is painted into.
//!
//! Before the framed transport is negotiated SWTOS speaks plain bytes, which
//! the decoder surfaces as `StreamItem::Plain` and which belong to channel 0,
//! the Shell pane. Channels, applications, and the debugger arrive with
//! framing; the desktop is already shaped for them.

use crate::keys;
use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_frontend::ui::{Cell, Desktop};
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

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

/// What the status line needs, gathered in one call so the session does not
/// grow an accessor per field.
pub struct Status {
    pub tick: u32,
    pub log_entries: usize,
    pub framed: bool,
    pub prefix_armed: bool,
}

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
            if self.decoder.mode() == Mode::Plain && self.tick >= self.next_hello {
                if let Ok(bytes) = hello().encode() {
                    self.uart.send(&bytes, FRAME_BYTE_CYCLES);
                }
                self.next_hello = self.tick.wrapping_add(HELLO_RETRY_TICKS);
            }
            self.uart
                .send(&heartbeat_frame(self.tick), HEARTBEAT_BYTE_CYCLES);
            self.tick = self.tick.wrapping_add(1);
            self.pump.run(&mut self.uart, BATCH);
            let output = self.uart.receive();
            for item in self.decoder.push(&output) {
                self.route(item);
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
    /// Typed input is echoed locally because SWTOS never echoes: without it
    /// typing looks broken while every byte is in fact arriving. Control bytes
    /// are filtered out of the echo -- pushing a raw Escape into a pane
    /// renders it as a replacement character.
    ///
    /// Prefix-`e` sends a real Escape to the focused application, which is how
    /// Uptime and Clock are stopped. It cannot simply be typed once copy mode
    /// and the help overlay also want Escape.
    pub fn send_key(&mut self, key: &str, ctrl: bool) -> Vec<u8> {
        if std::mem::take(&mut self.prefix_armed) {
            if key == "e" {
                self.transmit(&[0x1b]);
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
        let channel = self.desktop.focused_channel();
        self.desktop
            .push_channel(channel, &keys::echo_bytes(&bytes));
        self.transmit(&bytes);
        bytes
    }

    /// Place one decoded item on the desktop.
    ///
    /// Plain bytes are the pre-negotiation recovery transport and belong to
    /// the Shell. Framed TTY output is routed by channel, opening a pane for
    /// a channel not seen before. Frame kinds owned by later steps are left
    /// alone rather than silently dropped: an unhandled kind surfaces in the
    /// status line so a missing feature looks missing instead of broken.
    fn route(&mut self, item: StreamItem) {
        match item {
            StreamItem::Plain(bytes) => self.desktop.push_channel(SHELL, &bytes),
            StreamItem::Frame(frame) if frame.kind == FrameType::TtyOutput => {
                if !self.desktop.has_channel(frame.channel) {
                    self.desktop
                        .add_application(frame.channel, format!("TTY {}", frame.channel));
                }
                self.desktop.push_channel(frame.channel, &frame.payload);
            }
            StreamItem::Frame(frame) if frame.kind == FrameType::ChannelTitle => {
                self.desktop
                    .set_channel_title(frame.channel, String::from_utf8_lossy(&frame.payload));
            }
            StreamItem::Frame(frame) => self
                .desktop
                .set_error(Some(format!("unhandled frame {:?}", frame.kind))),
            StreamItem::Error(error) => self.desktop.set_error(Some(format!("{error:?}"))),
        }
    }

    /// Queue bytes for the target: raw before negotiation, and wrapped as a
    /// TTY_INPUT frame on the focused channel once framed.
    fn transmit(&mut self, bytes: &[u8]) {
        if self.decoder.mode() == Mode::Framed {
            let frame = Frame {
                kind: FrameType::TtyInput,
                channel: self.desktop.focused_channel(),
                payload: bytes.to_vec(),
            };
            if let Ok(encoded) = frame.encode() {
                self.uart.send(&encoded, FRAME_BYTE_CYCLES);
            }
            return;
        }
        self.uart.send(bytes, KEY_BYTE_CYCLES);
    }
}
