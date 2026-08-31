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
use crate::state::{Console, Panes};
use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
use swtos_frontend::resource::{Millis, ResourceSnapshot};
use swtos_frontend::ui::{Desktop, PaneKind};

/// Channel zero is the Shell pane, and is where unframed output belongs.
pub const SHELL: u8 = 0;

/// Suffix marking a pane whose process has exited. Carried in the title
/// rather than a side table: the title is already per-channel state the
/// desktop owns, so there is nothing to keep in sync.
pub const ENDED: &str = " (ended)";

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
            open_for_output(&mut panes.desktop, frame.channel);
            panes.desktop.push_channel(frame.channel, &frame.payload);
        }
        FrameType::ChannelOpen | FrameType::ChannelClose => {
            occupancy_changed(&mut panes.desktop, &frame);
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
        follow(&mut panes.desktop, current, &mut panes.seen_endpoints);
    }
}

/// Name each application pane after the process on its channel, and flag the
/// panes whose process has gone. Channel `n` carries endpoint `n + 1`.
pub fn follow(desktop: &mut Desktop, snapshot: &ResourceSnapshot, seen: &mut u32) {
    for (kind, channel, title) in desktop.layout() {
        if kind != PaneKind::Application {
            continue;
        }
        let endpoint = channel.saturating_add(1);
        let bit = 1u32 << u32::from(endpoint.min(31));
        match snapshot
            .processes
            .get(&endpoint)
            .filter(|process| process.state != 0)
        {
            Some(process) => {
                *seen |= bit;
                if !process.name.is_empty() && !title.starts_with(&process.name) {
                    desktop.set_channel_title(channel, process.name.clone());
                }
            }
            None => {
                if *seen & bit != 0 && !title.ends_with(ENDED) {
                    desktop.set_channel_title(channel, format!("{title}{ENDED}"));
                }
            }
        }
    }
}

/// A channel opened or closed.
fn occupancy_changed(desktop: &mut Desktop, frame: &Frame) {
    if frame.kind == FrameType::ChannelClose {
        if let Some((_, _, title)) = desktop
            .layout()
            .into_iter()
            .find(|(_, channel, _)| *channel == frame.channel)
            && !title.ends_with(ENDED)
        {
            desktop.set_channel_title(frame.channel, format!("{title}{ENDED}"));
        }
        return;
    }
    desktop.release_channel(frame.channel);
    let name = String::from_utf8_lossy(&frame.payload).into_owned();
    let title = if name.is_empty() {
        format!("TTY {}", frame.channel)
    } else {
        name
    };
    desktop.add_application(frame.channel, title);
}

/// Ensure a channel has a live pane before its output lands. A channel reused
/// after its process exited starts clean.
fn open_for_output(desktop: &mut Desktop, channel: u8) {
    let ended = desktop
        .layout()
        .into_iter()
        .any(|(_, ch, title)| ch == channel && title.ends_with(ENDED));
    if ended {
        desktop.release_channel(channel);
    }
    if !desktop.has_channel(channel) {
        desktop.add_application(channel, format!("TTY {channel}"));
    }
}
