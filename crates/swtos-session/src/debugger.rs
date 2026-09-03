//! The Debugger pane's local console.
//!
//! The debugger is not a TTY. Its pane owns channel 254, which SWTOS has no
//! terminal for, so typing there must be turned into DEBUG_REQUEST frames --
//! sending it as TTY_INPUT drops it on the floor, which is what "help does
//! nothing" looks like.

use crate::state::Console;
use swtos_frontend::debug::DebugConsole;
use swtos_frontend::debug::{DebugMap, identity_request};
use swtos_frontend::ui::Desktop;

/// The Debugger pane's channel, from `PaneKind::Debugger::default_channel`.
pub const CHANNEL: u8 = 254;

/// Greet, and ask the target to identify itself.
pub fn greet(desktop: &mut Desktop) -> Vec<u8> {
    desktop.push_channel(CHANNEL, b"SWTOS debugger\n");
    show_help(desktop);
    identity_request()
}

/// Put the debugger's own help in its pane.
///
/// Shown when the session opens and again whenever the target says it has been
/// rewound, because a restart or a reboot is exactly the moment someone is
/// looking for a way out with nothing else left on screen to go on.
pub fn show_help(desktop: &mut Desktop) {
    for line in swtos_frontend::debug::help_lines() {
        desktop.push_channel(CHANNEL, format!("{line}\n").as_bytes());
    }
}

/// Handle one key typed at the Debugger pane. Returns a DEBUG_REQUEST payload
/// when the command produced one.
///
/// A line starting `!` is handed to the shell instead, per
/// `docs/use-cases.md`: `!ps -l`, `!bg mon`, `!kill 3`.
pub fn key(console: &mut Console, desktop: &mut Desktop, key: &str) -> Option<Vec<u8>> {
    match key {
        "Enter" => enter(console, desktop),
        "Backspace" => {
            console.input.pop();
            desktop.push_channel(CHANNEL, &[0x08]);
            None
        }
        text if text.chars().count() == 1 => {
            console.input.push_str(text);
            desktop.push_channel(CHANNEL, text.as_bytes());
            None
        }
        _ => None,
    }
}

/// Feed a DEBUG_RESPONSE payload back into the pane.
pub fn response(console: &mut Console, desktop: &mut Desktop, payload: &[u8]) {
    console.awaiting = console.awaiting.saturating_sub(1);
    for line in console.console.response(payload) {
        desktop.push_channel(CHANNEL, format!("{line}\n").as_bytes());
    }
}

/// Install the debug map fetched at runtime, enabling `sym`, `list`, `dis`.
pub fn load_map(console: &mut Console, desktop: &mut Desktop, json: &str) {
    match DebugMap::from_json(json) {
        Ok(map) => {
            let build = map.build_id.clone();
            console.console.map = Some(map);
            desktop.push_channel(CHANNEL, format!("debug map loaded: {build}\n").as_bytes());
        }
        Err(error) => desktop.push_channel(CHANNEL, format!("debug map: {error}\n").as_bytes()),
    }
}

/// Run the line the user just finished typing.
fn enter(console: &mut Console, desktop: &mut Desktop) -> Option<Vec<u8>> {
    desktop.push_channel(CHANNEL, b"\n");
    if console.awaiting > 0 {
        desktop.push_channel(
            CHANNEL,
            b"(no reply to the previous request: the endpoint may hold no \
              process, or a runway process with no parked frame yet)\n",
        );
        console.awaiting = 0;
    }
    if let Some(command) = console.input.strip_prefix('!') {
        let command = command.trim().to_string();
        desktop.push_channel(CHANNEL, format!("-> shell: {command}\n").as_bytes());
        console.pending = Some(command);
        console.input.clear();
        return None;
    }
    let result = console.console.command(&console.input, None);
    console.input.clear();
    for line in result.lines {
        desktop.push_channel(CHANNEL, format!("{line}\n").as_bytes());
    }
    if result.request.is_some() {
        console.awaiting += 1;
    }
    result.request
}

/// A console with no debug map: at 1.6 MB it is fetched at runtime rather
/// than compiled in, so symbolic commands report its absence until it lands.
pub fn console() -> Console {
    Console {
        console: DebugConsole::new(None),
        input: String::new(),
        awaiting: 0,
        pending: None,
    }
}
