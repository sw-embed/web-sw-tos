//! Virtual UART behaviour. These live outside `src/` so the module itself
//! stays within the per-module function budget.

use swtos_host::uart::{FRAME_BYTE_CYCLES, HEARTBEAT_BYTE_CYCLES, VirtualUart};

#[test]
fn bytes_reach_the_target_in_order_with_their_cycle_budget() {
    let mut uart = VirtualUart::default();
    uart.send(b"hi", FRAME_BYTE_CYCLES);
    uart.send(&[0xff], HEARTBEAT_BYTE_CYCLES);

    assert_eq!(uart.next_for_target(), Some((b'h', FRAME_BYTE_CYCLES)));
    assert_eq!(uart.next_for_target(), Some((b'i', FRAME_BYTE_CYCLES)));
    assert_eq!(uart.next_for_target(), Some((0xff, HEARTBEAT_BYTE_CYCLES)));
    assert_eq!(uart.next_for_target(), None);
}

#[test]
fn receive_drains_and_leaves_the_queue_empty() {
    let mut uart = VirtualUart::default();
    uart.emit(b"out");
    assert_eq!(uart.receive(), b"out");
    assert!(uart.receive().is_empty());
}

/// The two directions are independent: the pty they replace was full duplex,
/// and a frame arriving while output is pending must not disturb it.
#[test]
fn the_two_directions_do_not_interfere() {
    let mut uart = VirtualUart::default();
    uart.emit(b"a");
    uart.send(b"b", FRAME_BYTE_CYCLES);
    assert_eq!(uart.receive(), b"a");
    assert_eq!(uart.next_for_target(), Some((b'b', FRAME_BYTE_CYCLES)));
}

/// Payload bytes are never escaped by the transport, so the sync sequence and
/// arbitrary binary must survive the queue untouched.
#[test]
fn binary_payloads_including_the_sync_sequence_pass_through_unchanged() {
    let raw: Vec<u8> = (0u8..=255).collect();
    let mut uart = VirtualUart::default();
    uart.emit(&raw);
    assert_eq!(uart.receive(), raw);

    uart.send(&[0xa5, 0x5a, 0x00, 0xff], FRAME_BYTE_CYCLES);
    let seen: Vec<u8> = std::iter::from_fn(|| uart.next_for_target())
        .map(|(byte, _)| byte)
        .collect();
    assert_eq!(seen, vec![0xa5, 0x5a, 0x00, 0xff]);
}
