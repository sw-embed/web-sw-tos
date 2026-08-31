Restructure toward the design sw-checklist is a forcing function for, rather than satisfying it by inlining.

Mike's correction: the limits exist to encourage functional programming, loose coupling, pure functions, delegation, design patterns, and include_ macros for non-code. Documented exceptions do not deliver readable, maintainable, testable code. Two defects in four steps came from doing the opposite -- inlining a guard into a larger function and collapsing accessors to reduce a count, which increased coupling and function length.

The evidence for the fix: both defects (the deleted CHANNEL_CLOSE arm, the dropped Mode::Plain HELLO guard) lived in code no native test could reach. session.rs and transport.rs are browser-bound only because they call js_sys::Date::now(). keys.rs and debugger.rs are already pure.

Work:

1. Inject the clock. Pass  (the type resource.rs already uses) into the routing and periodic paths instead of calling Date::now inside them. This alone makes session and transport pure.
2. Split by concern into crates so module budgets fall out of the design rather than being fought: browser glue (Yew, DOM measurement, page chrome) stays in the root crate; pure session logic, frame routing, and the debugger console move to their own crate; key translation is independent of both and can stand alone.
3. Port the browser-only behaviour to native tests now that it is reachable -- above all the HELLO retry guard, whose absence produced the menu loop, and the prefix state machine, whose modifier handling produced the dead Ctrl-A ?.

Acceptance: the HELLO guard and the prefix/modifier handling are covered by native tests that fail if either regresses. No self-authored sw-checklist failure and no new self-authored warning; where a count improves it must be because a concern moved to where it belongs, not because two functions were merged. clippy and fmt clean. The live demo behaves identically -- verify against the deployed URL.