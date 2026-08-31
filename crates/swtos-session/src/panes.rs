//! Pane bookkeeping: opening, ending, reuse, and who holds the keyboard.
//!
//! Separate from frame dispatch because it is a different job: nothing here
//! knows what a frame is, and `routing` does not know how a pane is named.

use swtos_frontend::protocol::{Frame, FrameType};
use swtos_frontend::resource::ResourceSnapshot;
use swtos_frontend::ui::{Desktop, PaneKind};

/// Suffix marking a pane whose process has exited. Carried in the title
/// rather than a side table: the title is already per-channel state the
/// desktop owns, so there is nothing to keep in sync.
pub const ENDED: &str = " (ended)";

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
pub fn occupancy_changed(desktop: &mut Desktop, frame: &Frame) {
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
    add_without_focus(desktop, frame.channel, title);
}

/// Open a pane for `channel` without moving the keyboard.
///
/// `add_application` sets focus to the pane it creates, so a launch would
/// steal the keyboard the instant its first output arrived and the next
/// keystroke would go to the new program instead of the Shell. te-rs avoids
/// this with `claim_application`, "reserve the next application TTY without
/// stealing keyboard focus"; upstream 525fbe2 is titled "a launch keeps the
/// keyboard". This is that, for output-driven panes.
fn add_without_focus(desktop: &mut Desktop, channel: u8, title: String) {
    let held = desktop.focused_channel();
    desktop.add_application(channel, title);
    restore_focus(desktop, held);
}

/// Put the keyboard back on the pane that had it.
fn restore_focus(desktop: &mut Desktop, channel: u8) {
    let Some(index) = desktop
        .layout()
        .into_iter()
        .position(|(_, pane, _)| pane == channel)
    else {
        return;
    };
    // The command table addresses panes as '1'..'9'; beyond that, step.
    if let Ok(digit) = u8::try_from(index)
        && digit < 9
    {
        desktop.command(b'1' + digit);
        return;
    }
    while desktop.focused_channel() != channel {
        desktop.command(b'n');
    }
}

/// Ensure a channel has a live pane before its output lands. A channel reused
/// after its process exited starts clean.
pub fn open_for_output(desktop: &mut Desktop, channel: u8) {
    let ended = desktop
        .layout()
        .into_iter()
        .any(|(_, ch, title)| ch == channel && title.ends_with(ENDED));
    if ended {
        desktop.release_channel(channel);
    }
    if !desktop.has_channel(channel) {
        add_without_focus(desktop, channel, format!("TTY {channel}"));
    }
}
