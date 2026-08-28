Negotiate the framed SWTOS transport. Nothing in Phase 2 works without this and no existing step covered it -- a genuine gap in the original plan, found while assessing completeness.

Today the demo never sends HELLO, so the target stays in plain recovery mode forever. Every byte arrives as StreamItem::Plain and lands in the Shell pane. That is why the Application, Debugger, and Resources panes render as empty boxes: no TTY_OUTPUT, CHANNEL_OPEN, RESOURCE_SNAPSHOT, or DEBUG_RESPONSE frame can ever arrive.

Per sw-tos docs/protocol.md: types 12 and 13 are HELLO and HELLO_ACK, channel zero, payload exactly 'SWT1'. Both peers stay in plain recovery mode until a valid ACK. The target accepts another exact HELLO while already framed, so re-negotiation is safe. protocol.rs already provides hello(), hello_ack(), Negotiator, and ConnectionDecoder handles the mode switch -- this step is about driving them, not writing them.

Deliverables:

- Send HELLO on startup and re-send periodically until Mode::Framed is reached, since the target may not be listening during early boot.
- Route framed items: TTY_OUTPUT by channel, and leave the other frame kinds to their own later steps rather than silently dropping them (log or surface unknown kinds).
- Keep plain mode working. It is the recovery transport and the boot banner arrives on it before negotiation completes; output already shown must not be lost or duplicated when the mode flips.
- Input must switch too: once framed, keystrokes go out as TTY_INPUT frames on the focused channel rather than as raw bytes.
- Surface the mode in the status line so it is obvious whether negotiation succeeded.

Acceptance: the live page reaches framed mode and the status line says so; the Shell still shows the boot banner from before negotiation; typing still drives the shell after the switch. Verify against the deployed URL, not just locally. clippy and fmt clean; no new self-authored sw-checklist findings.