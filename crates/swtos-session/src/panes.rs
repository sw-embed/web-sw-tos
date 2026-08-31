//! Pane bookkeeping: opening panes, and following the processes behind them.
//!
//! Much smaller than it was. Upstream now owns ended panes (`ended`,
//! `mark_live_endpoints`, `close_ended`), pane naming (`name_channel`),
//! clearing (`clear`, `clear_channel`), and no longer steals the keyboard when
//! a pane is added. Every local workaround for those has been deleted rather
//! than kept alongside; what remains is only what upstream does not do.

use swtos_frontend::protocol::{Frame, FrameType};
use swtos_frontend::resource::ResourceSnapshot;
use swtos_frontend::ui::Desktop;

/// Ensure a channel has a pane before its output lands.
pub fn open_for_output(desktop: &mut Desktop, channel: u8) {
    if !desktop.has_channel(channel) {
        desktop.add_application(channel, format!("TTY {channel}"));
    }
}

/// A channel opened or closed by the target.
///
/// A close is not what marks a pane ended -- SWTOS sends no CHANNEL_CLOSE, so
/// the resource snapshot is the authority and [`follow`] does it. This arm
/// exists because the frame is part of protocol v1 and another build may send
/// it.
pub fn occupancy_changed(desktop: &mut Desktop, frame: &Frame) {
    if frame.kind == FrameType::ChannelClose {
        return;
    }
    desktop.clear_channel(frame.channel);
    let name = String::from_utf8_lossy(&frame.payload).into_owned();
    let title = if name.is_empty() {
        format!("TTY {}", frame.channel)
    } else {
        name
    };
    desktop.add_application(frame.channel, title);
}

/// Name each pane after the process on its channel, and let the desktop mark
/// the ones whose process has gone.
pub fn follow(desktop: &mut Desktop, snapshot: &ResourceSnapshot) {
    let live: Vec<u8> = snapshot
        .processes
        .values()
        .filter(|process| process.state != 0)
        .map(|process| process.endpoint)
        .collect();
    for endpoint in &live {
        let Some(process) = snapshot.processes.get(endpoint) else {
            continue;
        };
        if let Some(channel) = endpoint.checked_sub(1) {
            desktop.name_channel(channel, &process.name);
        }
    }
    desktop.mark_live_endpoints(&live);
}
