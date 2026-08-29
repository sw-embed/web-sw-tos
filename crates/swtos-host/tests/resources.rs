//! The built-in monitor pane, fed by RESOURCE_SNAPSHOT frames.
//!
//! Distinct from the `mon` catalog program: this pane is the frontend's own,
//! assembled from bounded records the target sends on request. A generation
//! must land whole before it is published, so a partial one is never shown.

use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_frontend::resource::SnapshotAssembler;
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

#[test]
fn the_target_answers_a_resource_request_with_a_whole_generation() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut resources = SnapshotAssembler::default();
    let (mut published, mut now) = (Vec::new(), 0.0f64);

    for tick in 0..3000u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if decoder.mode() == Mode::Framed && tick.is_multiple_of(25) {
            let frame = Frame {
                kind: FrameType::ResourceSnapshot,
                channel: 0,
                payload: Vec::new(),
            };
            uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        now += 10.0;
        for item in decoder.push(&uart.receive()) {
            if let StreamItem::Frame(frame) = item
                && frame.kind == FrameType::ResourceSnapshot
                && resources.push(&frame.payload, now)
            {
                published = resources.render(now);
            }
        }
    }

    let text = published.join("\n");
    println!("--- monitor pane ---\n{text}");
    assert!(
        !published.is_empty(),
        "no complete generation was published"
    );
    assert!(
        text.contains("slots="),
        "memory line missing from the report: {text}"
    );
    assert!(
        text.contains("ep=1"),
        "no process rows in the report: {text}"
    );
    assert!(
        text.contains("uart rx="),
        "uart totals missing, so the generation was published early: {text}"
    );
}
