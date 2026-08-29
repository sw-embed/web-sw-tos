//! Everything between the byte queue and the desktop.
//!
//! Kept apart from the session so neither `run_until` nor `send_key` has to
//! carry framing details inline.
//!
//! Three routing decisions are worth stating once, here, rather than at the
//! arm that implements them:
//!
//! - An exited application keeps its pane, flagged `(ended)`. Upstream's
//!   `release_channel` would drop it, destroying the program's final output
//!   at the exact moment it becomes worth reading.
//! - Resource snapshots arrive as bounded records and are published only once
//!   a whole generation has landed; a partial one must never be shown.
//! - Negotiation frames are the decoder's business, not the desktop's.
//!   Reporting HelloAck as unhandled put a permanent "unhandled frame
//!   HelloAck" in the status line of every session that negotiated
//!   successfully -- an error message on the success path.

use crate::debugger::Console;
use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_frontend::resource::SnapshotAssembler;
use swtos_frontend::ui::Desktop;
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

/// Channel zero is the Shell pane, and is where unframed output belongs.
pub const SHELL: u8 = 0;

/// Place one decoded item on the desktop.
///
/// Plain bytes are the pre-negotiation recovery transport and belong to
/// the Shell. Framed TTY output is routed by channel, opening a pane for
/// a channel not seen before. Frame kinds owned by later steps are left
/// alone rather than silently dropped: an unhandled kind surfaces in the
/// status line so a missing feature looks missing instead of broken.
pub fn route(
    desktop: &mut Desktop,
    console: &mut Console,
    resources: &mut SnapshotAssembler,
    item: StreamItem,
) {
    match item {
        StreamItem::Plain(bytes) => desktop.push_channel(SHELL, &bytes),
        StreamItem::Frame(frame) if frame.kind == FrameType::TtyOutput => {
            open_for_output(desktop, frame.channel);
            desktop.push_channel(frame.channel, &frame.payload);
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::ChannelClose => {
            if let Some(title) = title_of(desktop, frame.channel)
                && !title.ends_with(ENDED)
            {
                desktop.set_channel_title(frame.channel, format!("{title}{ENDED}"));
            }
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::ChannelOpen => {
            desktop.release_channel(frame.channel);
            let name = String::from_utf8_lossy(&frame.payload).into_owned();
            let title = if name.is_empty() {
                format!("TTY {}", frame.channel)
            } else {
                name
            };
            desktop.add_application(frame.channel, title);
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::DebugResponse => {
            console.response(desktop, &frame.payload);
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::ResourceSnapshot => {
            let now = js_sys::Date::now();
            if resources.push(&frame.payload, now) {
                desktop.set_resources(&resources.render(now));
            }
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::ChannelTitle => {
            desktop.set_channel_title(frame.channel, String::from_utf8_lossy(&frame.payload));
        }
        StreamItem::Frame(frame)
            if matches!(frame.kind, FrameType::Hello | FrameType::HelloAck) => {}
        StreamItem::Frame(frame) => {
            desktop.set_error(Some(format!("unhandled frame {:?}", frame.kind)));
        }
        StreamItem::Error(error) => desktop.set_error(Some(format!("{error:?}"))),
    }
}

/// Send a DEBUG_REQUEST. Channel zero: the debugger is system-wide, not
/// bound to a virtual TTY.
pub fn request(uart: &mut VirtualUart, payload: Vec<u8>) {
    let frame = Frame {
        kind: FrameType::DebugRequest,
        channel: 0,
        payload,
    };
    if let Ok(encoded) = frame.encode() {
        uart.send(&encoded, FRAME_BYTE_CYCLES);
    }
}

/// Suffix marking a pane whose process has exited.
///
/// Deliberately carried in the title rather than in a side table: the title is
/// already per-channel state the desktop owns, so there is nothing to keep in
/// sync and nothing to leak when a pane is closed.
pub const ENDED: &str = " (ended)";

/// Ensure a channel has a live pane before its output lands.
///
/// A channel reused after its process exited starts clean, so one program's
/// output can never be read as the next one's. Dropping the pane and letting
/// it be recreated is the clear.
fn open_for_output(desktop: &mut Desktop, channel: u8) {
    if title_of(desktop, channel).is_some_and(|title| title.ends_with(ENDED)) {
        desktop.release_channel(channel);
    }
    if !desktop.has_channel(channel) {
        desktop.add_application(channel, format!("TTY {channel}"));
    }
}

/// The title a channel's pane currently carries, if it has one.
fn title_of(desktop: &Desktop, channel: u8) -> Option<String> {
    desktop
        .layout()
        .into_iter()
        .find(|(_, ch, _)| *ch == channel)
        .map(|(_, _, title)| title)
}

/// The most payload bytes the target's decoder will accept in one frame.
/// `docs/protocol.md`: the host accepts 1024, "the bounded COR24 decoder
/// accepts at most 16 bytes per frame". A longer frame is dropped in silence,
/// which looks exactly like a command being ignored.
const MAX_TARGET_PAYLOAD: usize = 16;

/// Queue bytes for the target: raw before negotiation, and wrapped as
/// TTY_INPUT frames on the focused channel once framed, split to the target's
/// payload bound.
pub fn transmit(uart: &mut VirtualUart, decoder: &ConnectionDecoder, channel: u8, bytes: &[u8]) {
    if decoder.mode() == Mode::Framed {
        for chunk in bytes.chunks(MAX_TARGET_PAYLOAD) {
            let frame = Frame {
                kind: FrameType::TtyInput,
                channel,
                payload: chunk.to_vec(),
            };
            if let Ok(encoded) = frame.encode() {
                uart.send(&encoded, FRAME_BYTE_CYCLES);
            }
        }
        return;
    }
    uart.send(bytes, HEARTBEAT_BYTE_CYCLES);
}

/// Everything the frontend sends on a timer once framed.
///
/// UPTIME and WALL_CLOCK carry a three-byte little-endian centisecond value
/// on channel zero. Without them Uptime reads a tick that never arrives and
/// counts erratically, and `mon` -- which refreshes on the uptime tick rather
/// than spinning -- never reports at all.
///
/// Uptime is derived from the scheduler tick, not the wall clock: one tick is
/// one centisecond by construction, so the figure stays consistent with the
/// heartbeat the target actually receives, even though emulated time runs
/// slower than real time. Wall clock is real, being centiseconds since local
/// midnight.
pub fn periodic(uart: &mut VirtualUart, desktop: &mut Desktop, tick: u32) {
    // The status line's clock is the frontend's own, and was never being set:
    // it read `--:--:--` for the life of every session.
    let now = js_sys::Date::new_0();
    desktop.set_clock(format!(
        "{:02}:{:02}:{:02}",
        now.get_hours(),
        now.get_minutes(),
        now.get_seconds()
    ));
    let wall_centiseconds = (f64::from(now.get_hours()) * 360_000.0
        + f64::from(now.get_minutes()) * 6_000.0
        + f64::from(now.get_seconds()) * 100.0) as u32;
    for (kind, value) in [
        (FrameType::Uptime, tick),
        (FrameType::WallClock, wall_centiseconds),
    ] {
        let frame = Frame {
            kind,
            channel: 0,
            payload: vec![value as u8, (value >> 8) as u8, (value >> 16) as u8],
        };
        if let Ok(encoded) = frame.encode() {
            uart.send(&encoded, FRAME_BYTE_CYCLES);
        }
    }
    // An empty RESOURCE_SNAPSHOT frame asks for a fresh generation; the reply
    // arrives as bounded records and feeds the built-in monitor pane.
    let request = Frame {
        kind: FrameType::ResourceSnapshot,
        channel: 0,
        payload: Vec::new(),
    };
    if let Ok(encoded) = request.encode() {
        uart.send(&encoded, FRAME_BYTE_CYCLES);
    }
}

/// Re-offer HELLO while still unnegotiated. The target is not listening during
/// early boot, so one attempt at startup is not enough.
pub fn negotiate(uart: &mut VirtualUart, decoder: &ConnectionDecoder) {
    if decoder.mode() == Mode::Plain
        && let Ok(bytes) = hello().encode()
    {
        uart.send(&bytes, FRAME_BYTE_CYCLES);
    }
}
