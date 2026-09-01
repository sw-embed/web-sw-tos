//! RESOURCE_SNAPSHOT frames, assembled.
//!
//! The frontend's own monitor pane is retired upstream -- `mon` reports as an
//! ordinary program now -- so the snapshot's remaining job is to say what is
//! running, which is what names panes and marks the ended ones. A generation
//! must land whole before it is published, so a partial one is never used.

use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_frontend::resource::SnapshotAssembler;
use swtos_host::control::heartbeat_frame;
use swtos_host::pump::Pump;
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

#[test]
fn the_target_answers_a_resource_request_with_a_whole_generation() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut resources = SnapshotAssembler::default();
    let (mut generations, mut now) = (0usize, 0.0f64);

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
                generations += 1;
            }
        }
    }

    let snapshot = resources
        .snapshot()
        .expect("no complete generation was published");

    println!(
        "generations: {generations}, slots {}/{}, uart rx={} tx={}",
        snapshot.memory.used_slots, snapshot.memory.total_slots, snapshot.uart_rx, snapshot.uart_tx
    );
    for process in snapshot.processes.values() {
        println!(
            "  ep={} name={} state={}",
            process.endpoint, process.name, process.state
        );
    }

    assert!(generations > 0, "no generation completed");
    assert!(
        snapshot.processes.contains_key(&1),
        "the shell is missing from the process table"
    );
    assert!(
        snapshot.uart_rx > 0,
        "uart totals are zero, so the generation was published before its \
         closing record arrived"
    );
    assert!(
        snapshot.memory.total_slots > 0,
        "the memory record never landed"
    );
}
