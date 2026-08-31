//! A launch keeps the keyboard.
//!
//! `add_application` focuses the pane it creates, so a program's first line of
//! output would steal the keyboard and the next keystroke would go to the new
//! program instead of the Shell. te-rs avoids this deliberately; upstream
//! 525fbe2 is titled "a launch keeps the keyboard".

use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
use swtos_session::state::Panes;
use swtos_session::{debugger, routing};

fn output(channel: u8, text: &str) -> StreamItem {
    StreamItem::Frame(Frame {
        kind: FrameType::TtyOutput,
        channel,
        payload: text.as_bytes().to_vec(),
    })
}

fn panes() -> Panes {
    Panes {
        desktop: Default::default(),
        resources: Default::default(),
        seen_endpoints: 0,
    }
}

#[test]
fn output_from_a_new_channel_does_not_steal_the_keyboard() {
    let (mut panes, mut console) = (panes(), debugger::console());
    let held = panes.desktop.focused_channel();

    routing::route(&mut panes, &mut console, 0.0, output(4, "hello\n"));

    assert_eq!(
        panes.desktop.focused_channel(),
        held,
        "a launch moved the keyboard to the new pane, so the next keystroke \
         would go to the program instead of the Shell"
    );
    assert!(panes.desktop.has_channel(4), "the pane was not opened");
}

/// A channel the target opens explicitly must not steal it either.
#[test]
fn an_opened_channel_does_not_steal_the_keyboard() {
    let (mut panes, mut console) = (panes(), debugger::console());
    let held = panes.desktop.focused_channel();

    routing::route(
        &mut panes,
        &mut console,
        0.0,
        StreamItem::Frame(Frame {
            kind: FrameType::ChannelOpen,
            channel: 5,
            payload: b"counter".to_vec(),
        }),
    );

    assert_eq!(
        panes.desktop.focused_channel(),
        held,
        "CHANNEL_OPEN stole focus"
    );
}
