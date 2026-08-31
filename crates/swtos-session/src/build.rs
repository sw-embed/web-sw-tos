//! Construction. Kept apart from [`crate::state`] so the declarations there
//! stay free of behaviour.

use crate::state::{Console, Input, Panes, Session, Transport};
use swtos_frontend::debug::DebugConsole;
use swtos_host::pump::Pump;

/// A session with the vendored image loaded and nothing negotiated yet.
///
/// The debug map is `None`: at 1.6 MB it is fetched at runtime rather than
/// compiled in, so symbolic commands report its absence until it arrives.
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
        console: console(),
        tick: 0,
        greeted: false,
    }
}

pub fn console() -> Console {
    Console {
        console: DebugConsole::new(None),
        input: String::new(),
        awaiting: 0,
        pending: None,
    }
}
