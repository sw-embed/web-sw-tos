Fix GitHub issue #1: the Ctrl-O prefix works in a browser on macOS but not on
Chrome/Linux, leaving the whole frontend command set unreachable there.

The cause is a platform asymmetry, not a bug in the dispatch code. On macOS the
browser's own shortcuts are Cmd-based, so every Ctrl combination falls through
to the page. On Linux and Windows they are Ctrl-based, and Ctrl-O is Open File
-- handled by the browser before the page sees the keydown, so preventDefault
cannot reach it. Every prefix binding is dead on those platforms: no pane
switching, no zoom, no help, no restart.

Two fixes, and the issue asks for both ("and/or"). Do both: they fail
independently, which is the point.

1. A better default prefix. Ctrl-B is tmux's own prefix, is not a browser
   shortcut on any platform, encodes to 0x02, and is neither CR nor LF (the two
   sw-tos refuses). Keep it single-sourced through dispatch::PREFIX_KEY and
   PREFIX_LABEL so the status line, the help overlay and the arming code cannot
   name different keys -- that drift is what sw-tos f9197df was fixing when it
   moved off Ctrl-A. Diverging from upstream's Ctrl-O is legitimate here for the
   same reason `Instant` and `std::fs` diverge: a platform constraint the CLI
   does not have. Record it in the vendored patch inventory.

2. A command menu reachable with the mouse, which is the fix that cannot be
   defeated by a browser shortcut. A button in the page chrome opens a menu of
   the prefix commands; clicking one dispatches exactly as if the prefix had
   been pressed first, through the same dispatch::key path rather than a second
   copy of the command table.

The 'no mouse' rule in CLAUDE.md is about the character screen, not the page:
the geometry selector has always been a mouse control, on the stated grounds
that page chrome is not a terminal control. The menu is page chrome. Say so in
CLAUDE.md rather than leaving the rule looking violated.

Keep the command table as data next to dispatch, so the menu and the key
handler cannot disagree about what a key does. Respect the module budgets:
web-sw-tos is at four modules and chrome.rs at six functions.

Test the dispatch path natively -- a menu click and a keypress must produce the
same result. Then verify in a real browser that the prefix arms and a menu
button works.