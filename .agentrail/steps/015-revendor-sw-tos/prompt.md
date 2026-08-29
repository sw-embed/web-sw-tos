Re-vendor from sw-tos d6dbce9. Sixteen commits landed since the last vendoring and several change what the browser demo can do.

Upstream changes that matter here:
- b8fd430 sixteen process slots (was three), and a TTY that can hold a command line
- aae34ea every process gets its own protocol channel -- this changes channel routing, which the browser now implements
- 08f0cfd process stacks and the loaded-image heap moved into SRAM; 482f030 reports heap use
- e29306f debugger gains a 'map' command with three memory views
- 81cc4f0 'list' no longer resolves addresses outside the image -- the exact wart flagged while explaining why dis fee7db failed
- 482f030 Escape is echoed into the pane it is sent to
- edbe921 every clock and uptime process ticks, not just the foreground one
- 24ee640 / d6dbce9 menu choice 9 fills every process slot, led by two cpu-hogs
- 71ff551 te-rs clippy findings cleared upstream

Good news to verify first, not assume: protocol.rs and docs/protocol.md are unchanged, so the wire format, HELLO negotiation, and the 16-byte payload bound still hold.

Work:
1. Re-vendor assets/program.bin and program.debug.json via scripts/refresh-image.sh, then update the identity constants in crates/swtos-host/src/image.rs and assets/PROVENANCE.md.
2. Re-vendor ui.rs, resource.rs, debug.rs from d6dbce9. protocol.rs is unchanged; confirm rather than recopy blindly.
3. Re-apply only the REQUIRED patches from the inventory in docs/architecture.md: Instant -> Millis, load(path) -> from_json, and the Cell grid with render_grid. Check each clippy patch against upstream first and DELETE the row from the inventory if upstream fixed it -- do not keep a local patch alive merely because it exists.
4. Update the inventory table and provenance headers to d6dbce9.

Acceptance: all vendored tests pass unmodified; the negotiate tests still pass (they encode real target behaviour, so a failure there is a genuine upstream change worth reporting, not a test to paper over); clippy and fmt clean; no self-authored sw-checklist failure. Verify the live demo still boots, negotiates, and runs a command.

Note: Mike says the CLI TUI demo upstream is not fully stable yet, so treat an upstream oddity as information rather than something to chase.