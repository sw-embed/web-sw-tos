//! Pane lifecycle.
//!
//! Almost all of this is upstream behaviour now: the desktop owns the `ended`
//! flag, marks it from the live endpoint list, names panes after their
//! process, clears on demand, and reclaims finished panes. These tests drive
//! the routing that feeds it, and assert the outcome a viewer sees.

use swtos_frontend::protocol::{Frame, FrameType, StreamItem};
use swtos_frontend::resource::{ProcessSnapshot, ResourceSnapshot};
use swtos_frontend::ui::Desktop;
use swtos_session::state::{Console, Panes};
use swtos_session::{debugger, panes, routing};

fn output(channel: u8, text: &str) -> StreamItem {
    StreamItem::Frame(Frame {
        kind: FrameType::TtyOutput,
        channel,
        payload: text.as_bytes().to_vec(),
    })
}

fn workspace() -> (Panes, Console) {
    (
        Panes {
            desktop: Desktop::default(),
            resources: Default::default(),
        },
        debugger::console(),
    )
}

fn screen(panes: &Panes) -> String {
    panes
        .desktop
        .render_grid(140, 50)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A snapshot listing exactly these endpoints as running.
fn snapshot(live: &[(u8, &str)]) -> ResourceSnapshot {
    let mut snapshot = ResourceSnapshot::default();
    for (endpoint, name) in live {
        snapshot.processes.insert(
            *endpoint,
            ProcessSnapshot {
                endpoint: *endpoint,
                state: 1,
                name: (*name).into(),
                ..Default::default()
            },
        );
    }
    snapshot
}

/// Output opens a pane, and the snapshot names it after its process.
#[test]
fn a_pane_is_named_after_the_process_on_its_channel() {
    let (mut panes, mut console) = workspace();
    routing::route(&mut panes, &mut console, 0.0, output(3, "working\n"));
    panes::follow(&mut panes.desktop, &snapshot(&[(4, "hello")]));

    let text = screen(&panes);
    assert!(
        text.contains("hello"),
        "pane not named from the snapshot: {text}"
    );
    assert!(text.contains("working"), "output lost: {text}");
}

/// When the process goes, the pane stays and says so -- the program's final
/// output is what becomes worth reading at exactly that moment.
#[test]
fn a_pane_says_when_its_process_has_ended() {
    let (mut panes, mut console) = workspace();
    routing::route(
        &mut panes,
        &mut console,
        0.0,
        output(3, "the answer is 42\n"),
    );
    panes::follow(&mut panes.desktop, &snapshot(&[(4, "hello")]));

    // One absent snapshot is not proof. Output can reach a pane before the
    // snapshot that first lists its process as running, and a pane that
    // flickered to (ended) while its program was starting up would be worse
    // than one that takes a moment to notice.
    panes::follow(&mut panes.desktop, &snapshot(&[]));
    assert!(
        !screen(&panes).contains("(ended)"),
        "one missing snapshot was treated as proof the process had gone"
    );

    panes::follow(&mut panes.desktop, &snapshot(&[]));
    let text = screen(&panes);
    assert!(
        text.contains("(ended)"),
        "an exited process left no mark: {text}"
    );
    assert!(
        text.contains("the answer is 42"),
        "ending the pane destroyed its final output: {text}"
    );
}

/// A pane that never held a process is not a dead one.
#[test]
fn an_idle_pane_is_not_marked_ended() {
    let (mut panes, _console) = workspace();
    panes::follow(&mut panes.desktop, &snapshot(&[]));
    assert!(
        !screen(&panes).contains("(ended)"),
        "an idle pane was marked ended"
    );
}

/// A channel the target reopens starts clean, so one program's output cannot
/// be read as the next one's.
#[test]
fn reopening_a_channel_clears_it() {
    let (mut panes, mut console) = workspace();
    routing::route(&mut panes, &mut console, 0.0, output(3, "from the first\n"));
    routing::route(
        &mut panes,
        &mut console,
        0.0,
        StreamItem::Frame(Frame {
            kind: FrameType::ChannelOpen,
            channel: 3,
            payload: b"counter".to_vec(),
        }),
    );

    let text = screen(&panes);
    assert!(
        !text.contains("from the first"),
        "the reopened pane still held the previous program's output: {text}"
    );
}

/// Opening a pane must not move the keyboard.
#[test]
fn opening_a_pane_keeps_the_keyboard() {
    let (mut panes, mut console) = workspace();
    let held = panes.desktop.focused_channel();
    routing::route(&mut panes, &mut console, 0.0, output(4, "hello\n"));
    assert_eq!(
        panes.desktop.focused_channel(),
        held,
        "a launch moved the keyboard to the new pane"
    );
}
