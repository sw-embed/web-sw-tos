//! Does SWTOS actually boot under the pump, and fast enough?
//!
//! This is the acceptance gate for the whole browser port. If the image does
//! not run here it cannot run in WASM either, and these numbers are the
//! native baseline the browser is measured against.

use std::time::Instant;
use swtos_host::control::heartbeat_frame;
use swtos_host::pump::Pump;
use swtos_host::uart::{HEARTBEAT_BYTE_CYCLES, VirtualUart};

const BATCH: u64 = 50_000;

#[test]
fn swtos_boots_and_writes_to_the_uart() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let mut output = Vec::new();
    for tick in 0..200 {
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        output.extend(uart.receive());
    }
    let text: String = output.iter().map(|b| *b as char).collect();
    println!("--- {} bytes of UART output ---\n{text}", output.len());
    assert!(!output.is_empty(), "SWTOS produced no UART output at all");
}

#[test]
fn native_throughput_baseline() {
    let (mut pump, mut uart) = (Pump::default(), VirtualUart::default());
    let ticks = 500u32;
    let start = Instant::now();
    for tick in 0..ticks {
        uart.send(&heartbeat_frame(tick), HEARTBEAT_BYTE_CYCLES);
        pump.run(&mut uart, BATCH);
        uart.receive();
    }
    let elapsed = start.elapsed();
    // Each tick spends 5 heartbeat bytes plus one batch.
    let cycles = u64::from(ticks) * (5 * HEARTBEAT_BYTE_CYCLES + BATCH);
    let per_sec = cycles as f64 / elapsed.as_secs_f64();
    println!(
        "{ticks} ticks in {elapsed:?}  ->  {:.1}M cycles/sec, {:.2} ms/tick",
        per_sec / 1e6,
        elapsed.as_secs_f64() * 1000.0 / f64::from(ticks)
    );
    println!(
        "uart log holds {} entries after {ticks} ticks",
        pump.log_len()
    );
}
