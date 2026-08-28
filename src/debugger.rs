//! The Debugger pane's local console.
//!
//! The debugger is not a TTY. Its pane owns channel 254, which SWTOS has no
//! terminal for, so typing there must be handled here and turned into
//! DEBUG_REQUEST frames -- sending it as TTY_INPUT drops it on the floor,
//! which is exactly what "help does nothing" looks like.

use crate::transport;
use swtos_frontend::debug::{DebugConsole, identity_request};
use swtos_frontend::ui::{Desktop, PaneKind};
use swtos_host::uart::VirtualUart;

/// The Debugger pane's channel, from `PaneKind::Debugger::default_channel`.
pub const CHANNEL: u8 = 254;

pub struct Console {
    console: DebugConsole,
    input: String,
    /// Requests sent with no reply yet. The target answers an endpoint that
    /// holds no process, or a runway process with no parked ISR frame, with
    /// silence -- indistinguishable from a broken debugger unless said.
    awaiting: usize,
}

impl Default for Console {
    fn default() -> Self {
        // No debug map: at 1.6 MB it is not compiled into the bundle, so
        // symbolic commands (sym, list, dis) report that rather than lying.
        // Everything that talks to the target -- regs, x, kill -- works.
        Self {
            console: DebugConsole::new(None),
            input: String::new(),
            awaiting: 0,
        }
    }
}

impl Console {
    /// Greet, and ask the target to identify itself. Returns the request to
    /// send once the transport is framed.
    pub fn greet(&mut self, desktop: &mut Desktop) -> Vec<u8> {
        desktop.push_channel(CHANNEL, b"SWTOS debugger: type help\n");
        identity_request()
    }

    /// Handle one key typed at the Debugger pane. Returns a DEBUG_REQUEST
    /// payload when the command produced one.
    pub fn key(&mut self, desktop: &mut Desktop, key: &str) -> Option<Vec<u8>> {
        match key {
            "Enter" => {
                desktop.push_channel(CHANNEL, b"\n");
                if self.awaiting > 0 {
                    desktop.push_channel(
                        CHANNEL,
                        b"(no reply to the previous request: the endpoint may hold no \
                          process, or a runway process with no parked frame yet)\n",
                    );
                    self.awaiting = 0;
                }
                let result = self.console.command(&self.input);
                self.input.clear();
                for line in result.lines {
                    desktop.push_channel(CHANNEL, format!("{line}\n").as_bytes());
                }
                if result.request.is_some() {
                    self.awaiting += 1;
                }
                result.request
            }
            "Backspace" => {
                self.input.pop();
                desktop.push_channel(CHANNEL, &[0x08]);
                None
            }
            text if text.chars().count() == 1 => {
                self.input.push_str(text);
                desktop.push_channel(CHANNEL, text.as_bytes());
                None
            }
            _ => None,
        }
    }

    /// Feed a DEBUG_RESPONSE payload back into the pane.
    pub fn response(&mut self, desktop: &mut Desktop, payload: &[u8]) {
        self.awaiting = self.awaiting.saturating_sub(1);
        for line in self.console.response(payload) {
            desktop.push_channel(CHANNEL, format!("{line}\n").as_bytes());
        }
    }
}

impl Console {
    /// Give a local-console pane first refusal on a key, returning true when
    /// it took it.
    pub fn consume(
        &mut self,
        kind: PaneKind,
        desktop: &mut Desktop,
        uart: &mut VirtualUart,
        key: &str,
    ) -> bool {
        match kind {
            PaneKind::Debugger => {
                if let Some(request) = self.key(desktop, key) {
                    transport::request(uart, request);
                }
                true
            }
            PaneKind::Resources => true,
            _ => false,
        }
    }
}
