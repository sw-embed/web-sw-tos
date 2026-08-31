//! Inbound frames to panes.
//!
//! Three routing decisions are worth stating once, here, rather than at the
//! arm that implements them:
//!
//! - An exited application keeps its pane, flagged `(ended)`. Dropping it
//!   would destroy the program's final output at the moment it becomes worth
//!   reading. CHANNEL_CLOSE is not what detects the exit -- SWTOS never sends
//!   one, measured rather than assumed -- so the resource snapshot is the
//!   authority and [`follow_processes`] does the flagging.
//! - Resource snapshots arrive as bounded records and are published only once
//!   a whole generation has landed; a partial one must never be shown.
//! - Negotiation frames are the decoder's business, not the desktop's.
//!   Reporting HelloAck as unhandled put a permanent error in the status line
//!   of every session that negotiated successfully.

use crate::debugger;
use crate::panes;
use crate::state::{Console, Panes};
use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
use swtos_frontend::resource::Millis;

/// Channel zero is the Shell pane, and is where unframed output belongs.
pub const SHELL: u8 = 0;

/// Place one decoded item on the desktop.
pub fn route(panes: &mut Panes, console: &mut Console, now: Millis, item: StreamItem) {
    match item {
        StreamItem::Plain(bytes) => panes.desktop.push_channel(SHELL, &bytes),
        StreamItem::Frame(frame) => frame_arrived(panes, console, now, frame),
        StreamItem::Error(error) => panes.desktop.set_error(Some(format!("{error:?}"))),
    }
}

/// One decoded frame.
fn frame_arrived(panes: &mut Panes, console: &mut Console, now: Millis, frame: Frame) {
    match frame.kind {
        FrameType::TtyOutput => {
            panes::open_for_output(&mut panes.desktop, frame.channel);
            panes.desktop.push_channel(frame.channel, &frame.payload);
        }
        FrameType::ChannelOpen | FrameType::ChannelClose => {
            panes::occupancy_changed(&mut panes.desktop, &frame);
        }
        FrameType::DebugResponse => {
            debugger::response(console, &mut panes.desktop, &frame.payload);
        }
        FrameType::ResourceSnapshot => snapshot(panes, now, &frame.payload),
        FrameType::ChannelTitle => {
            panes
                .desktop
                .set_channel_title(frame.channel, String::from_utf8_lossy(&frame.payload));
        }
        FrameType::Hello | FrameType::HelloAck => {}
        kind => panes
            .desktop
            .set_error(Some(format!("unhandled frame {kind:?}"))),
    }
}

/// A resource record. Published only once a whole generation has landed.
fn snapshot(panes: &mut Panes, now: Millis, payload: &[u8]) {
    if !panes.resources.push(payload, now) {
        return;
    }
    let lines = panes.resources.render(now);
    panes.desktop.set_resources(&lines);
    if let Some(current) = panes.resources.snapshot() {
        panes::follow(&mut panes.desktop, current, &mut panes.seen_endpoints);
    }
}
