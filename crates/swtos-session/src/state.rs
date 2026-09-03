//! Data declarations.
//!
//! Structures only: no `impl` blocks and no behaviour. The functions that act
//! on these live in the modules named after the job they do, which keeps each
//! module about one thing and leaves the shape of the data readable on its
//! own.
//!
//! [`Session`] is composed of smaller pieces rather than being one flat
//! record, so each behaviour module borrows only the part it needs: routing
//! never sees the UART, sending never sees the panes.

use swtos_frontend::debug::DebugConsole;
use swtos_frontend::protocol::ConnectionDecoder;
use swtos_frontend::resource::Millis;
use swtos_frontend::resource::SnapshotAssembler;
use swtos_frontend::ui::Desktop;
use swtos_host::pump::Pump;
use swtos_host::uart::VirtualUart;

/// The byte path to the target: the wire and what is known about its mode.
pub struct Transport {
    pub uart: VirtualUart,
    pub decoder: ConnectionDecoder,
    /// Tick at which the next HELLO goes out, while still unnegotiated.
    pub next_hello: u32,
}

/// Everything shown on screen, and what is known about the processes behind
/// it.
pub struct Panes {
    pub desktop: Desktop,
    pub resources: SnapshotAssembler,
}

/// Keyboard state that spans more than one key.
pub struct Input {
    /// Set by the prefix key; the next key is a frontend command, not input.
    pub prefix_armed: bool,
}

/// The Debugger pane's local console. Not a TTY.
pub struct Console {
    pub console: DebugConsole,
    pub input: String,
    /// Requests sent with no reply yet. The target answers an endpoint holding
    /// no process, or a runway process with no parked frame, with silence.
    pub awaiting: usize,
    /// A `!command` waiting to be handed to the shell.
    pub pending: Option<String>,
    /// When to print the prompt that a reply owes.
    ///
    /// The target answers asynchronously and in as many frames as it likes --
    /// registers arrive in two -- so a prompt printed straight after the
    /// command lands under the reply, and the next line typed has none,
    /// because the one owed to it was already spent.
    pub prompt_due: Option<Millis>,
}

/// One live SWTOS session.
pub struct Session {
    pub pump: Pump,
    pub transport: Transport,
    pub panes: Panes,
    pub input: Input,
    pub console: Console,
    pub tick: u32,
    /// Set once the debugger has greeted, after the transport goes framed.
    pub greeted: bool,
}

/// What the status line reports.
pub struct Status {
    pub tick: u32,
    pub log_entries: usize,
    pub framed: bool,
    pub prefix_armed: bool,
}

/// Local wall-clock time, to the second.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocalTime {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

/// The two questions this project asks of a clock.
///
/// A trait rather than a concrete type so the browser supplies one
/// implementation and tests supply another. Reading `js_sys::Date` inside the
/// routing and driver paths is what previously made them browser-only, and
/// what let a dropped HELLO guard and a swallowed prefix key ship unnoticed:
/// neither could be reached from a native test.
pub trait Clock {
    /// Monotonic milliseconds, for deadlines and staleness.
    fn elapsed(&self) -> Millis;

    /// Local time, for the status line and the wall-clock tick.
    fn local(&self) -> LocalTime;
}
