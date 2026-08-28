//! Everything between the byte queue and the desktop.
//!
//! Kept apart from the session so neither `run_until` nor `send_key` has to
//! carry framing details inline.

use crate::debugger::Console;
use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
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
pub fn route(desktop: &mut Desktop, console: &mut Console, item: StreamItem) {
    match item {
        StreamItem::Plain(bytes) => desktop.push_channel(SHELL, &bytes),
        StreamItem::Frame(frame) if frame.kind == FrameType::TtyOutput => {
            if !desktop.has_channel(frame.channel) {
                desktop.add_application(frame.channel, format!("TTY {}", frame.channel));
            }
            desktop.push_channel(frame.channel, &frame.payload);
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::DebugResponse => {
            console.response(desktop, &frame.payload);
        }
        StreamItem::Frame(frame) if frame.kind == FrameType::ChannelTitle => {
            desktop.set_channel_title(frame.channel, String::from_utf8_lossy(&frame.payload));
        }
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

/// Queue bytes for the target: raw before negotiation, and wrapped as a
/// TTY_INPUT frame on the focused channel once framed.
pub fn transmit(uart: &mut VirtualUart, decoder: &ConnectionDecoder, channel: u8, bytes: &[u8]) {
    if decoder.mode() == Mode::Framed {
        let frame = Frame {
            kind: FrameType::TtyInput,
            channel,
            payload: bytes.to_vec(),
        };
        if let Ok(encoded) = frame.encode() {
            uart.send(&encoded, FRAME_BYTE_CYCLES);
        }
        return;
    }
    uart.send(bytes, HEARTBEAT_BYTE_CYCLES);
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
