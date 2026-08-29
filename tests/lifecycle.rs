//! Pane lifecycle: what happens when an application ends, when its channel is
//! reused, and when the user clears a pane.
//!
//! These are deliberate divergences from te-rs, decided in docs/plan.md.
//! Upstream drops a pane on ChannelClose, which destroys the program's final
//! output at the moment it becomes worth reading, and reuses a channel without
//! clearing it, which lets one program's output be read as the next one's.

use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
use swtos_frontend::ui::Desktop;
use web_sw_tos::debugger::Console;
use web_sw_tos::transport::{self, ENDED};

fn output(channel: u8, text: &str) -> StreamItem {
    StreamItem::Frame(Frame {
        kind: FrameType::TtyOutput,
        channel,
        payload: text.as_bytes().to_vec(),
    })
}

fn close(channel: u8) -> StreamItem {
    StreamItem::Frame(Frame {
        kind: FrameType::ChannelClose,
        channel,
        payload: Vec::new(),
    })
}

fn screen(desktop: &Desktop) -> String {
    desktop
        .render_grid(120, 43)
        .into_iter()
        .map(|row| row.into_iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn feed(desktop: &mut Desktop, items: Vec<StreamItem>) {
    let mut console = Console::default();
    let mut resources = swtos_frontend::resource::SnapshotAssembler::default();
    for item in items {
        transport::route(desktop, &mut console, &mut resources, item);
    }
}

/// An ended application keeps its pane and its output, flagged.
#[test]
fn a_closed_channel_keeps_its_pane_and_is_marked_ended() {
    let mut desktop = Desktop::default();
    feed(
        &mut desktop,
        vec![output(3, "important result\n"), close(3)],
    );

    let text = screen(&desktop);
    assert!(
        text.contains("important result"),
        "closing the channel destroyed the program's output: {text}"
    );
    assert!(
        text.contains(ENDED.trim()),
        "an ended pane was not flagged: {text}"
    );
}

/// Closing twice must not stack the suffix.
#[test]
fn ending_is_idempotent() {
    let mut desktop = Desktop::default();
    feed(&mut desktop, vec![output(3, "x\n"), close(3), close(3)]);
    let text = screen(&desktop);
    assert!(
        !text.contains("(ended) (ended)"),
        "the ended suffix stacked: {text}"
    );
}

/// Reusing an ended channel starts clean, so one program's output cannot be
/// mistaken for the next one's.
#[test]
fn reusing_an_ended_channel_clears_the_pane() {
    let mut desktop = Desktop::default();
    feed(
        &mut desktop,
        vec![output(3, "from the first program\n"), close(3)],
    );
    feed(&mut desktop, vec![output(3, "from the second\n")]);

    let text = screen(&desktop);
    assert!(
        text.contains("from the second"),
        "new output missing: {text}"
    );
    assert!(
        !text.contains("from the first program"),
        "the reused pane still held the previous program's output: {text}"
    );
    assert!(
        !text.contains(ENDED.trim()),
        "the reused pane is still flagged ended: {text}"
    );
}

/// A live channel is never cleared just because more output arrives.
#[test]
fn a_live_channel_keeps_accumulating() {
    let mut desktop = Desktop::default();
    feed(
        &mut desktop,
        vec![output(3, "first\n"), output(3, "second\n")],
    );
    let text = screen(&desktop);
    assert!(text.contains("first") && text.contains("second"), "{text}");
}

/// Ctrl-A c empties the focused pane and nothing else.
#[test]
fn clear_empties_only_the_focused_pane() {
    let mut desktop = Desktop::default();
    feed(&mut desktop, vec![output(3, "application output\n")]);
    desktop.push_channel(0, b"shell output\n");

    desktop.command(b'1'); // focus the Shell
    desktop.clear_focused();

    let text = screen(&desktop);
    assert!(!text.contains("shell output"), "focused pane not cleared");
    assert!(
        text.contains("application output"),
        "clear reached an unfocused pane: {text}"
    );
}

/// A form feed replaces a pane's contents instead of appending to them.
///
/// `mon` redraws a whole report each refresh and leads it with 0x0c. Without
/// this the reports stack and the pane scrolls, which is what the browser did
/// while the CLI replaced in place; worse, 0x0c fell through to the
/// replacement-character arm, so every refresh also left visible garbage.
#[test]
fn a_form_feed_replaces_the_pane_rather_than_appending() {
    let mut desktop = Desktop::default();
    feed(&mut desktop, vec![output(3, "\x0cfirst report\n")]);
    feed(&mut desktop, vec![output(3, "\x0csecond report\n")]);

    let text = screen(&desktop);
    assert!(
        text.contains("second report"),
        "latest report missing: {text}"
    );
    assert!(
        !text.contains("first report"),
        "the form feed did not clear the pane, so reports stacked: {text}"
    );
    assert!(
        !text.contains('\u{fffd}'),
        "the form feed rendered as a replacement character: {text}"
    );
}
