//! Does `kill EP` actually terminate a process?
use swtos_frontend::debug::DebugConsole;
use swtos_frontend::protocol::{ConnectionDecoder, Frame, FrameType, Mode, StreamItem, hello};
use swtos_host::control::heartbeat_frame;
use swtos_host::pump::Pump;
use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

#[test]
fn kill_a_spawned_process() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut decoder = ConnectionDecoder::default();
    let mut console = DebugConsole::new(None);
    let (mut shell, mut dbg) = (String::new(), Vec::new());
    let mut closes = Vec::new();
    let mut resources = swtos_frontend::resource::SnapshotAssembler::default();
    let mut now = 0.0f64;
    let (mut spawned, mut killed, mut checked) = (false, false, false);

    let send = |uart: &mut VirtualUart, kind, payload: Vec<u8>| {
        for chunk in payload.chunks(16) {
            let frame = Frame {
                kind,
                channel: 0,
                payload: chunk.to_vec(),
            };
            uart.send(&frame.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
    };

    for tick in 0..9000u32 {
        if decoder.mode() == Mode::Plain && tick % 25 == 0 {
            uart.send(&hello().encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if decoder.mode() == Mode::Framed && tick.is_multiple_of(25) {
            let f = Frame {
                kind: FrameType::Uptime,
                channel: 0,
                payload: vec![tick as u8, (tick >> 8) as u8, (tick >> 16) as u8],
            };
            uart.send(&f.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if decoder.mode() == Mode::Framed && !spawned {
            send(
                &mut uart,
                FrameType::TtyInput,
                b"run mon --tty=new\n".to_vec(),
            );
            spawned = true;
        }
        if spawned && !killed && tick > 3000 {
            send(&mut uart, FrameType::DebugRequest, vec![13, 2]);
            killed = true;
        }
        if decoder.mode() == Mode::Framed && tick.is_multiple_of(50) {
            let f = Frame {
                kind: FrameType::ResourceSnapshot,
                channel: 0,
                payload: Vec::new(),
            };
            uart.send(&f.encode().expect("bounded"), FRAME_BYTE_CYCLES);
        }
        if killed && !checked && tick > 6000 {
            send(&mut uart, FrameType::TtyInput, b"ps -l\n".to_vec());
            checked = true;
        }
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        now += 10.0;
        for item in decoder.push(&uart.receive()) {
            if let StreamItem::Frame(frame) = item {
                match frame.kind {
                    FrameType::TtyOutput if frame.channel == 0 => {
                        shell.extend(frame.payload.iter().map(|b| *b as char))
                    }
                    FrameType::DebugResponse => dbg.extend(console.response(&frame.payload)),
                    FrameType::ChannelClose => closes.push(frame.channel),
                    FrameType::TtyOutput if frame.channel != 0 => {
                        if !closes.contains(&frame.channel) {
                            closes.push(frame.channel);
                        }
                    }
                    FrameType::ResourceSnapshot => {
                        resources.push(&frame.payload, now);
                    }
                    _ => {}
                }
            }
        }
    }
    println!("--- debugger said ---\n{dbg:#?}");
    let ps = shell
        .lines()
        .rfind(|line| line.starts_with("ep=2"))
        .unwrap_or("(no ep=2 row)");
    println!("--- final ep=2 row ---\n{ps}");
    println!("--- non-zero channels that carried output: {closes:?}");
    if let Some(snap) = resources.snapshot() {
        let live: Vec<(u8, &str, u8)> = snap
            .processes
            .values()
            .map(|p| (p.endpoint, p.name.as_str(), p.state))
            .collect();
        println!("--- endpoints in the final snapshot: {live:?}");
    }
    // The frontend's monitor pane is retired upstream; the snapshot itself is
    // now the record of what is running.
    let live: Vec<String> = resources
        .snapshot()
        .map(|snap| {
            snap.processes
                .values()
                .filter(|process| process.state != 0)
                .map(|process| format!("ep={} {}", process.endpoint, process.name))
                .collect()
        })
        .unwrap_or_default();
    println!("--- running after kill ---\n{live:?}");

    assert!(
        dbg.iter().any(|line| line.contains("kill requested")),
        "the target refused the kill: {dbg:?}"
    );
    // Either shape means gone. sw-tos e08fa4e made a released slot forget
    // what ran in it, so `ps -l` now omits the row entirely where it used to
    // show the old name with state=0.
    assert!(
        ps.contains("state=0") || ps == "(no ep=2 row)" || ps.contains("name=none"),
        "endpoint 2 is still running after kill: {ps}"
    );
    assert!(
        !live.iter().any(|row| row.starts_with("ep=2 ")),
        "a killed process is still listed as running: {live:?}"
    );
    assert!(
        closes.is_empty() || !closes.contains(&0),
        "unexpected close on the shell channel"
    );
}
