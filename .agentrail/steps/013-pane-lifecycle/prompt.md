Implement the pane lifecycle decisions recorded in docs/plan.md under 'Pane lifecycle, and a deliberate divergence from te-rs'. This runs after 012-terminal-grid and before 013-application-panes wires the channel frames, so the behaviour exists before there are panes to exercise it.

Three changes to the vendored ui.rs, each a deliberate divergence from te-rs that must be marked with a comment saying so and why:

1. ChannelClose must NOT remove an application pane. Keep it, suffix its title with ' (ended)', and mark it so it no longer accepts input. Removing the pane destroys the program's final output at the moment it becomes worth reading. Ctrl-A x must still close it.

2. Reusing a channel for a new application clears the pane first (scrollback, partial line, scroll offset), so one program's output can never be read as the next one's. Today add_application reuses and retitles but leaves the old text.

3. Add a clear command: Ctrl-A c clears the focused pane. te-rs has none, so the only way to discard output is closing the pane. Add it to the help overlay text too.

Acceptance: tests for each of the three, driven through the Desktop API rather than the renderer -- an ended pane keeps its lines and gains the suffix, a reused channel starts empty, and clear empties the focused pane only. clippy -D warnings and fmt clean; no NEW sw-checklist findings beyond the documented vendored-protocol.rs exception (ui.rs is vendored, so it inherits that exception, but say so explicitly in the commit).