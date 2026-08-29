//! Time ticks, and the `mon` resource monitor that depends on them.
//!
//! `mon` refreshes on the uptime tick rather than spinning, so without
//! UPTIME frames it never reports at all -- and Uptime reads a tick that
//! never arrives and counts erratically.

use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

/// Run `command`, optionally feeding the uptime tick, and return what each
/// channel printed.
fn session(command: &[u8], time_ticks: bool) -> Vec<(u8, String)> {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let (mut out, mut sent) = (Vec::new(), false);

    for tick in 0..6000u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if decoder.mode() == Mode::Framed && !sent {
            for byte in command.chunks(16) {
                let frame = Frame {
                    kind: FrameType::TtyInput,
                    channel: 0,
                    payload: byte.to_vec(),
                };
                uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
            }
            sent = true;
        }
        if time_ticks && decoder.mode() == Mode::Framed && tick.is_multiple_of(25) {
            let frame = Frame {
                kind: FrameType::Uptime,
                channel: 0,
                payload: vec![tick as u8, (tick >> 8) as u8, (tick >> 16) as u8],
            };
            uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        for item in decoder.push(&uart.receive()) {
            if let StreamItem::Frame(frame) = item
                && frame.kind == FrameType::TtyOutput
            {
                out.push((
                    frame.channel,
                    frame.payload.iter().map(|b| *b as char).collect::<String>(),
                ));
            }
        }
    }
    let mut by_channel: std::collections::BTreeMap<u8, String> = Default::default();
    for (ch, text) in out {
        by_channel.entry(ch).or_default().push_str(&text);
    }
    by_channel.into_iter().collect()
}

/// `mon` is new upstream: a resource monitor that runs as an ordinary catalog
/// process, so it gets its own pane and can be killed like anything else.
#[test]
fn mon_reports_when_the_uptime_tick_arrives() {
    let with_ticks: String = session(b"run mon --tty=new\n", true)
        .into_iter()
        .map(|(_, text)| text)
        .collect();
    assert!(
        with_ticks.contains("mem") || with_ticks.contains("ep="),
        "mon produced no resource report: {with_ticks}"
    );
}

/// Uptime counts up. Without the tick it reads a value that never arrives.
#[test]
fn uptime_counts_monotonically_when_ticked() {
    let text: String = session(b"3", true).into_iter().map(|(_, t)| t).collect();
    let stamps: Vec<u32> = text
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            let (m, s) = line.split_once(':')?;
            Some(m.parse::<u32>().ok()? * 60 + s.parse::<u32>().ok()?)
        })
        .collect();
    assert!(stamps.len() >= 3, "too few timestamps to judge: {text:?}");
    assert!(
        stamps.windows(2).all(|w| w[1] >= w[0]),
        "uptime went backwards: {stamps:?}"
    );
}
