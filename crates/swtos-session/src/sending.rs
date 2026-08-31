//! Everything the frontend puts on the wire.
//!
//! Separate from the run loop so the loop reads as a sequence of steps and
//! each thing that can be sent is one small function with its own reason.

use crate::debugger;
use crate::state::{Clock, LocalTime, Session};
use swtos_frontend::protocol::{Frame, FrameType, Mode, hello};
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES};

/// Ticks between HELLO attempts while still in plain mode.
const HELLO_RETRY: u32 = 25;

/// Ticks between time frames and resource requests.
const PERIODIC: u32 = 25;

/// The most payload bytes the target's decoder accepts in one frame. A longer
/// frame is dropped in silence, which looks exactly like a command ignored.
const MAX_PAYLOAD: usize = 16;

/// Queue bytes for the target: raw before negotiation, framed after, split to
/// the target's payload bound.
pub fn to_channel(session: &mut Session, channel: u8, bytes: &[u8]) {
    if session.transport.decoder.mode() != Mode::Framed {
        session.transport.uart.send(bytes, HEARTBEAT_BYTE_CYCLES);
        return;
    }
    for chunk in bytes.chunks(MAX_PAYLOAD) {
        frame(session, FrameType::TtyInput, channel, chunk.to_vec());
    }
}

/// Send a DEBUG_REQUEST. Channel zero: the debugger is system-wide.
pub fn debug_request(session: &mut Session, payload: Vec<u8>) {
    frame(session, FrameType::DebugRequest, 0, payload);
}

/// Re-offer HELLO **only while unnegotiated**.
///
/// The mode guard is load-bearing, not defensive: the target accepts a repeat
/// HELLO while framed and treats it as a fresh attach, re-running catalog
/// autostart and reprinting the menu on every retry.
pub fn offer_hello(session: &mut Session) {
    if session.transport.decoder.mode() != Mode::Plain
        || session.tick < session.transport.next_hello
    {
        return;
    }
    if let Ok(bytes) = hello().encode() {
        session.transport.uart.send(&bytes, FRAME_BYTE_CYCLES);
    }
    session.transport.next_hello = session.tick.wrapping_add(HELLO_RETRY);
}

/// Time ticks, the resource request, and the first greeting, once framed.
pub fn periodic(session: &mut Session, clock: &impl Clock) {
    if session.transport.decoder.mode() != Mode::Framed {
        return;
    }
    greet_once(session);
    if !session.tick.is_multiple_of(PERIODIC) {
        return;
    }
    let (display, centiseconds) = wall(clock.local());
    session.panes.desktop.set_clock(display);
    for (kind, value) in [
        (FrameType::Uptime, session.tick),
        (FrameType::WallClock, centiseconds),
    ] {
        let payload = vec![value as u8, (value >> 8) as u8, (value >> 16) as u8];
        frame(session, kind, 0, payload);
    }
    frame(session, FrameType::ResourceSnapshot, 0, Vec::new());
}

/// Greet in the Debugger pane the first time the transport goes framed.
fn greet_once(session: &mut Session) {
    if session.greeted {
        return;
    }
    session.greeted = true;
    session.panes.desktop.set_error(None);
    let request = debugger::greet(&mut session.panes.desktop);
    debug_request(session, request);
}

/// The two forms the wall clock is needed in: text for the status line, and
/// centiseconds since midnight for the WALL_CLOCK frame. One function because
/// both come from the same reading for the same purpose.
fn wall(time: LocalTime) -> (String, u32) {
    let seconds =
        u32::from(time.hours) * 3600 + u32::from(time.minutes) * 60 + u32::from(time.seconds);
    (
        format!("{:02}:{:02}:{:02}", time.hours, time.minutes, time.seconds),
        seconds * 100,
    )
}

fn frame(session: &mut Session, kind: FrameType, channel: u8, payload: Vec<u8>) {
    let frame = Frame {
        kind,
        channel,
        payload,
    };
    if let Ok(encoded) = frame.encode() {
        session.transport.uart.send(&encoded, FRAME_BYTE_CYCLES);
    }
}
