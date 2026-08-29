Re-vendor sw-tos 1e75960 and send the time ticks that make it useful.

New upstream: 1e75960 adds 'mon', a resource monitor that runs as an ordinary catalog process. It reports the same figures as the built-in Resources pane but can be spawned several times, each getting its own pane, and killed like anything else. 3ba48fd lets the debugger kill any process but the shell. 707405d renames panes on the rule above them (ui.rs, 196 lines changed). protocol.rs and docs/protocol.md are unchanged for the third cycle running -- confirm rather than assume.

The blocker to make mon work: it refreshes on the uptime tick, and this project has never sent time frames at all. That is also why Uptime showed non-monotonic output (00:06, 00:00, 00:06) -- it was reading a tick that never arrived.

Work:
1. Re-vendor assets (image is now 1f1480bf...) and ui.rs; resource.rs and debug.rs were untouched upstream, so verify before recopying.
2. Send UPTIME (type 6) and WALL_CLOCK (type 7) frames on channel 0 while framed, payload a three-byte little-endian centisecond value. Derive uptime from the scheduler tick count, not the wall clock: one tick is one centisecond by construction, so it stays self-consistent with the heartbeat even though emulated time runs slower than real time. Wall clock is centiseconds since local midnight.
3. Verify natively that 'run mon' produces repeating output, and that Uptime now counts monotonically.

Acceptance: mon spawns and refreshes in the live demo, with its own pane. Uptime counts up without going backwards. clippy and fmt clean, no self-authored sw-checklist failure.