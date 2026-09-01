//! Drives the emulated COR24 and moves bytes across the virtual UART.
//!
//! This is the browser's counterpart to `cor24-debug-adapter`'s main loop.
//! The adapter runs a process that sleeps 1 ms between batches; here the
//! caller supplies the cadence, because in a browser the clock belongs to the
//! event loop rather than to us.

use crate::image;
use crate::uart::VirtualUart;
use cor24_emulator::EmulatorCore;
use cor24_emulator::cpu::state::UartDirection;

/// The emulator plus the bookkeeping needed to read its UART as bytes.
pub struct Pump {
    emu: EmulatorCore,
    log_seen: usize,
}

impl Default for Pump {
    /// Load the vendored image exactly as the command-line adapter does.
    /// SWTOS owns memory outside the emulator's default standalone stack
    /// window, so the stack bounds are cleared rather than left at their
    /// defaults.
    fn default() -> Self {
        let mut emu = EmulatorCore::new();
        emu.load_program(0, image::PROGRAM);
        emu.set_pc(0);
        emu.set_stack_bounds(0, 0);
        Self { emu, log_seen: 0 }
    }
}

impl Pump {
    /// Deliver everything queued for the target, honouring each byte's cycle
    /// budget, then run `cycles` more and hand back whatever the target
    /// emitted. Input is paced because a SWTOS input ring holds 16 bytes and
    /// silently drops overruns.
    pub fn run(&mut self, uart: &mut VirtualUart, cycles: u64) {
        while let Some((byte, budget)) = uart.next_for_target() {
            self.emu.send_uart_byte(byte);
            self.emu.resume();
            self.emu.run_batch(budget);
        }
        self.emu.resume();
        self.emu.run_batch(cycles);
        // `get_uart_output` cannot be used here: it stores each byte as
        // `value as char` and skips NUL, so it is lossy for the binary framed
        // transport. The log is the byte-exact source.
        let entries = self.emu.uart_log().entries();
        let output: Vec<u8> = entries[self.log_seen..]
            .iter()
            .filter(|entry| entry.direction == UartDirection::Output)
            .map(|entry| entry.byte)
            .collect();
        self.log_seen = entries.len();
        uart.emit(&output);
    }

    /// Number of entries the emulator's UART log holds. It grows without
    /// bound and has no public clear, so this is the handle on that cost.
    pub fn log_len(&self) -> usize {
        self.emu.uart_log().entries().len()
    }
}
