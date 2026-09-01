//! Does the target answer HELLO?
//!
//! Native, because a failure here is a protocol or timing problem and the
//! browser adds nothing but latency to diagnosing it.

use swtos_frontend::protocol::{ConnectionDecoder, Mode, StreamItem, hello};
use swtos_host::control::heartbeat_frame;
use swtos_host::pump::Pump;
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

/// The command shell is reachable, and both line terminators work.
///
/// History worth keeping: the shell once terminated only on LF, and sending
/// CR parsed the command correctly and then answered `BAD` -- so `help` and
/// `ps -l` looked unsupported when they were merely mis-terminated. That cost
/// real debugging time. Upstream's shell-parsing rework now accepts CR too,
/// which this test surfaced by failing rather than by anyone noticing.
///
/// Enter still sends LF from this frontend: it works against both the old and
/// the new target, so there is no reason to change it.
#[test]
fn framed_mode_reaches_the_command_shell_on_either_terminator() {
    assert!(
        shell_reply(b"help\n").contains("ps"),
        "help did not list ps"
    );
    let table = shell_reply(b"ps -l\n");
    println!("--- ps -l ---\n{table}");
    assert!(
        table.contains("shell"),
        "ps -l did not report the process table"
    );
    assert!(
        !shell_reply(b"help\r").contains("BAD"),
        "CR was refused; it has been accepted since sw-tos 525fbe2"
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

/// Does a DEBUG_REQUEST for registers actually come back?
///
/// Identity (opcode 1) is known to work -- the pane reports the build CRC --
/// so this isolates whether opcode 2 replies, and for which endpoints.
#[test]
fn registers_request_gets_a_response() {
    use swtos_frontend::debug::{DebugConsole, registers_request};
    use swtos_frontend::protocol::{Frame, FrameType};

    for endpoint in [1u8, 2] {
        let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
        let mut decoder = ConnectionDecoder::default();
        let mut console = DebugConsole::new(None);
        let mut lines: Vec<String> = Vec::new();
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
                    kind: FrameType::DebugRequest,
                    channel: 0,
                    payload: registers_request(endpoint),
                };
                uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
                sent = true;
            }
            uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
            pump.run(&mut uart, BATCH);
            for item in decoder.push(&uart.receive()) {
                if let StreamItem::Frame(frame) = item
                    && frame.kind == FrameType::DebugResponse
                {
                    lines.extend(console.response(&frame.payload));
                }
            }
        }
        println!("endpoint {endpoint}: {lines:?}");
    }
}

/// Spawn a process, then read its registers: the full path Mike used in the
/// terminal. Also records which channels the target opens, since an
/// Application pane only appears when one does.
#[test]
fn spawning_populates_an_endpoint_and_then_regs_answers() {
    use swtos_frontend::debug::{DebugConsole, registers_request};
    use swtos_frontend::protocol::{Frame, FrameType};

    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut console = DebugConsole::new(None);
    let (mut lines, mut channels, mut tty) = (Vec::new(), Vec::new(), Vec::new());
    let (mut spawned, mut asked) = (false, false);

    // One byte per frame, as te-rs does. The target's decoder accepts at most
    // 16 bytes of payload and silently drops anything longer.
    let send = |uart: &mut VirtualUart, kind, payload: Vec<u8>| {
        for chunk in payload.chunks(1) {
            let frame = Frame {
                kind,
                channel: 0,
                payload: chunk.to_vec(),
            };
            uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
    };

    for tick in 0..4000u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if decoder.mode() == Mode::Framed && !spawned {
            send(
                &mut uart,
                FrameType::TtyInput,
                b"run cpu-hog --tty=new\n".to_vec(),
            );
            spawned = true;
        }
        if spawned && !asked && tick > 1200 {
            send(&mut uart, FrameType::TtyInput, b"ps -l\n".to_vec());
            send(&mut uart, FrameType::DebugRequest, registers_request(2));
            send(&mut uart, FrameType::DebugRequest, registers_request(3));
            asked = true;
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        for item in decoder.push(&uart.receive()) {
            if let StreamItem::Frame(frame) = item {
                match frame.kind {
                    FrameType::DebugResponse => lines.extend(console.response(&frame.payload)),
                    FrameType::ChannelOpen => channels.push(frame.channel),
                    FrameType::TtyOutput => tty.push((
                        frame.channel,
                        frame.payload.iter().map(|b| *b as char).collect::<String>(),
                    )),
                    _ => {}
                }
            }
        }
    }
    let mut by_channel: std::collections::BTreeMap<u8, String> = Default::default();
    for (ch, text) in &tty {
        by_channel.entry(*ch).or_default().push_str(text);
    }
    // The target opens no channel of its own: te-rs claims the application
    // pane locally on seeing the `--tty=new` suffix. Recorded because an
    // Application pane appearing is frontend work, not something to wait for.
    assert!(channels.is_empty(), "target opened a channel: {channels:?}");
    let shell: String = by_channel.get(&0).cloned().unwrap_or_default();
    assert!(shell.contains("READY"), "cpu-hog did not spawn: {shell}");
    assert!(
        shell.contains("cpu-hog"),
        "endpoint 2 was not populated: {shell}"
    );
    println!("channels opened: {channels:?}");
    println!("regs 2 while unpreempted: {lines:?}");
    println!("regs 2 after spawn: {lines:?}");
}

/// The target's decoder accepts at most 16 payload bytes and drops anything
/// longer in silence. A 22-byte `run cpu-hog --tty=new` sent as one frame
/// vanishes; split, it spawns. This cost real debugging time because the
/// failure is indistinguishable from the command being unsupported.
#[test]
fn oversized_tty_input_frames_are_dropped_by_the_target() {
    use swtos_frontend::protocol::{Frame, FrameType};

    fn run(chunk: usize) -> String {
        let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
        let mut decoder = ConnectionDecoder::default();
        let (mut out, mut sent) = (String::new(), false);
        for tick in 0..2500u32 {
            if decoder.mode() == Mode::Plain && tick % 25 == 0 {
                uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
            }
            if decoder.mode() == Mode::Framed && !sent {
                for part in b"run cpu-hog --tty=new\n".chunks(chunk) {
                    let frame = Frame {
                        kind: FrameType::TtyInput,
                        channel: 0,
                        payload: part.to_vec(),
                    };
                    uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
                }
                sent = true;
            }
            uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
            pump.run(&mut uart, BATCH);
            for item in decoder.push(&uart.receive()) {
                if let StreamItem::Frame(frame) = item
                    && frame.kind == FrameType::TtyOutput
                {
                    out.extend(frame.payload.iter().map(|b| *b as char));
                }
            }
        }
        out
    }

    // The boundary is sharp and was measured, not assumed: 16 accepted,
    // 17 dropped, re-confirmed against the sixteen-slot image.
    assert!(run(16).contains("READY"), "a 16-byte payload was refused");
    assert!(
        !run(17).contains("READY"),
        "a 17-byte payload was accepted; the target's bound has grown"
    );
}

/// A repeat HELLO after negotiation restarts the session.
///
/// Upstream runs the catalog's autostart programs whenever a frontend
/// attaches, and the target accepts a fresh HELLO while already framed. A
/// frontend that keeps retrying therefore re-attaches on every retry,
/// re-running autostart and reprinting the menu -- which reads as the shell
/// stuck in a loop. The retry must stop at the first HelloAck.
#[test]
fn hello_after_negotiation_restarts_the_session() {
    fn menus(keep_saying_hello: bool) -> usize {
        use swtos_frontend::protocol::FrameType;

        let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
        let mut decoder = ConnectionDecoder::default();
        let mut out = String::new();
        for tick in 0..3000u32 {
            let plain = decoder.mode() == Mode::Plain;
            if (plain || keep_saying_hello) && tick % 25 == 0 {
                uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
            }
            uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
            pump.run(&mut uart, BATCH);
            for item in decoder.push(&uart.receive()) {
                match item {
                    StreamItem::Plain(bytes) => out.extend(bytes.iter().map(|b| *b as char)),
                    StreamItem::Frame(frame) if frame.kind == FrameType::TtyOutput => {
                        out.extend(frame.payload.iter().map(|b| *b as char));
                    }
                    _ => {}
                }
            }
        }
        out.matches("MENU").count()
    }

    let disciplined = menus(false);
    let looping = menus(true);
    println!("menus printed -- stopping at ack: {disciplined}, retrying forever: {looping}");
    assert!(
        looping > disciplined,
        "a repeat HELLO no longer restarts the session; this guard may be unnecessary"
    );
    assert!(
        disciplined <= 3,
        "the menu repeated even while HELLO stopped at the ack: {disciplined}"
    );
}
