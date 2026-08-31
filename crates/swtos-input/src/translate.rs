//! Browser key events to terminal bytes and frontend commands.
//!
//! Kept apart from the session so the translation is pure and testable: every
//! case here fails silently when it is wrong, because a key that produces no
//! bytes looks exactly like an emulator that is not running.

use swtos_frontend::ui::Desktop;

/// Keys that are modifiers in their own right. A browser delivers these as
/// their own keydown before the character they produce, which a terminal
/// never does.
pub fn is_modifier(key: &str) -> bool {
    matches!(
        key,
        "Shift" | "Control" | "Alt" | "Meta" | "CapsLock" | "AltGraph"
    )
}

/// The bytes a real terminal would send for this key, or empty if the key is
/// not one the target should see.
///
/// Ctrl-A through Ctrl-Z collapse to 0x01..0x1a exactly as a terminal encodes
/// them. Named keys such as "Shift" or "ArrowUp" are longer than one
/// character and are dropped rather than typed into the shell as words.
pub fn to_bytes(key: &str, ctrl: bool) -> Vec<u8> {
    match (key, ctrl) {
        // LF, not CR. The SWTOS shell terminates every command on byte 10:
        // sending 13 parses the command correctly and then answers BAD, which
        // is why `help` and `ps -l` appeared unsupported.
        ("Enter", _) => vec![b'\n'],
        ("Backspace", _) => vec![0x08],
        ("Tab", _) => vec![b'\t'],
        ("Escape", _) => vec![0x1b],
        (name, true) if name.len() == 1 => match name.as_bytes()[0].to_ascii_uppercase() {
            c @ b'A'..=b'Z' => vec![c - b'A' + 1],
            _ => Vec::new(),
        },
        (text, false) if text.chars().count() == 1 => text.as_bytes().to_vec(),
        _ => Vec::new(),
    }
}

/// What the frontend shows for bytes the user typed. SWTOS never echoes, so
/// without this typing looks broken while every byte is in fact arriving.
/// Control bytes are dropped: pushing a raw Escape into a pane renders it as a
/// replacement character.
pub fn echo_bytes(bytes: &[u8]) -> Vec<u8> {
    bytes
        .iter()
        .filter_map(|byte| match byte {
            b'\n' | 0x08 | 0x20..=0x7e => Some(*byte),
            _ => None,
        })
        .collect()
}

/// The byte `Desktop::command` expects for a prefix command. Named keys map to
/// their terminal equivalents so Escape and Tab work as commands too.
pub fn command_byte(key: &str) -> u8 {
    match key {
        "Escape" => 0x1b,
        "Tab" => b'\t',
        text if text.chars().count() == 1 => text.as_bytes()[0],
        _ => 0,
    }
}

/// Drive copy mode. Returns false for keys copy mode does not claim, so the
/// caller can fall through. Mirrors the CLI bindings: arrows or `hjkl` by one
/// line, `PgUp`/`PgDn` or `u`/`d` by ten, `g`/`G` to either end, `q` to leave.
pub fn copy_motion(desktop: &mut Desktop, key: &str) -> bool {
    match key {
        "ArrowUp" | "k" => desktop.copy_move(1, 0),
        "ArrowDown" | "j" => desktop.copy_move(-1, 0),
        "ArrowLeft" | "h" => desktop.copy_move(0, -1),
        "ArrowRight" | "l" => desktop.copy_move(0, 1),
        "PageUp" | "u" => desktop.copy_move(10, 0),
        "PageDown" | "d" => desktop.copy_move(-10, 0),
        "g" => desktop.copy_home(),
        "G" => desktop.copy_end(),
        "q" | "Escape" => {
            desktop.command(b'y');
        }
        _ => return false,
    }
    true
}
