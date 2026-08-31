//! Advancing the emulator.

use crate::debugger;
use crate::routing;
use crate::sending;
use crate::state::{Clock, Input, Panes, Session, Status, Transport};
use swtos_frontend::protocol::Mode;
use swtos_frontend::resource::Millis;
use swtos_host::pump::Pump;
use swtos_host::uart::HEARTBEAT_BYTE_CYCLES;

/// Cycles to run per tick beyond the heartbeat's own pacing.
const BATCH: u64 = 50_000;

/// Advance at most `steps` scheduler ticks, stopping once the clock passes
/// `deadline`. Returns how many ran.
pub fn run(session: &mut Session, steps: u32, deadline: Millis, clock: &impl Clock) -> u32 {
    let mut done = 0;
    while done < steps {
        sending::offer_hello(session);
        sending::periodic(session, clock);
        let heartbeat = swtos_host::pump::heartbeat_frame(session.tick);
        session
            .transport
            .uart
            .send(&heartbeat, HEARTBEAT_BYTE_CYCLES);
        session.tick = session.tick.wrapping_add(1);
        session.pump.run(&mut session.transport.uart, BATCH);
        let output = session.transport.uart.receive();
        let now = clock.elapsed();
        for item in session.transport.decoder.push(&output) {
            routing::route(&mut session.panes, &mut session.console, now, item);
        }
        done += 1;
        if clock.elapsed() >= deadline {
            break;
        }
    }
    done.max(1)
}

/// Install the runtime-fetched debug map.
pub fn load_map(session: &mut Session, json: &str) {
    debugger::load_map(&mut session.console, &mut session.panes.desktop, json);
}

/// Everything the status line reports.
pub fn status(session: &Session) -> Status {
    Status {
        tick: session.tick,
        log_entries: session.pump.log_len(),
        framed: session.transport.decoder.mode() == Mode::Framed,
        prefix_armed: session.input.prefix_armed,
    }
}

/// A session with the vendored image loaded and nothing negotiated yet.
pub fn session() -> Session {
    Session {
        pump: Pump::default(),
        transport: Transport {
            uart: Default::default(),
            decoder: Default::default(),
            next_hello: 0,
        },
        panes: Panes {
            desktop: Default::default(),
            resources: Default::default(),
            seen_endpoints: 0,
        },
        input: Input {
            prefix_armed: false,
        },
        console: crate::debugger::console(),
        tick: 0,
        greeted: false,
    }
}
