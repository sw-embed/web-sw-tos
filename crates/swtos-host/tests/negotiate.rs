//! Does the target answer HELLO?
//!
//! Native, because a failure here is a protocol or timing problem and the
//! browser adds nothing but latency to diagnosing it.

use swtos_frontend::protocol::{ConnectionDecoder, Mode, StreamItem, hello};
use swtos_host::pump::{Pump, heartbeat_frame};
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

#[test]
fn the_target_acknowledges_hello_and_the_transport_goes_framed() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut plain = Vec::new();
    let mut frames = 0usize;

    for tick in 0..600u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(
                &hello().encode().expect("HELLO is bounded"),
                FRAME_BYTE_CYCLES,
            );
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        for item in decoder.push(&uart.receive()) {
            match item {
                StreamItem::Plain(bytes) => plain.extend(bytes),
                StreamItem::Frame(_) => frames += 1,
                StreamItem::Error(_) => {}
            }
        }
        if decoder.mode() == Mode::Framed && frames > 0 {
            break;
        }
    }

    let banner: String = plain.iter().map(|b| *b as char).collect();
    println!(
        "mode={:?} frames={frames}\nplain banner:\n{banner}",
        decoder.mode()
    );
    assert_eq!(
        decoder.mode(),
        Mode::Framed,
        "target never acknowledged HELLO; still plain after 600 ticks"
    );
}

/// The command shell documented in `windows-usage.md` is reachable, and each
/// command is terminated by LF.
///
/// This pins a bug that cost real debugging time: the shell parses every
/// command byte by byte and terminates on 10. Sending 13 parses the command
/// correctly and then answers `BAD`, so `help` and `ps -l` looked unsupported
/// when they were merely mis-terminated. A CR regression would be invisible
/// without this test -- the target still replies, just with a refusal.
#[test]
fn framed_mode_reaches_the_command_shell_and_commands_end_with_lf() {
    assert!(
        shell_reply(b"help\n").contains("ps"),
        "help did not list ps"
    );
    assert!(
        shell_reply(b"ps -l\n").contains("name=shell"),
        "ps -l did not report the process table"
    );
    assert!(
        shell_reply(b"help\r").contains("BAD"),
        "CR was accepted; the LF requirement this test pins has changed"
    );
}

/// Negotiate, send one command, and return everything the target replied.
fn shell_reply(command: &[u8]) -> String {
    use swtos_frontend::protocol::{Frame, FrameType};

    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut output = Vec::new();
    let mut sent = false;

    for tick in 0..1500u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(
                &hello().encode().expect("HELLO is bounded"),
                FRAME_BYTE_CYCLES,
            );
        }
        if decoder.mode() == Mode::Framed && !sent {
            let frame = Frame {
                kind: FrameType::TtyInput,
                channel: 0,
                payload: command.to_vec(),
            };
            uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
            sent = true;
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        for item in decoder.push(&uart.receive()) {
            if let StreamItem::Frame(frame) = item
                && frame.kind == FrameType::TtyOutput
            {
                output.extend(frame.payload);
            }
        }
    }
    output.iter().map(|b| *b as char).collect()
}
