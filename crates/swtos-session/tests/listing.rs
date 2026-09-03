//! `list` in a browser, where there is no assembly beside the map.
//!
//! sw-tos 99af617 made `list` show source rather than a second disassembly, by
//! reading the `.s` files next to the map. Neither exists here: the map is
//! fetched as a static asset and the assembly is not shipped at all. Upstream
//! anticipated that -- `source_dir` is `None` and it falls back to the one line
//! the map itself holds -- and this pins that the browser gets that fallback
//! rather than an error, an empty pane, or a reach at a filesystem.

mod fake;

use fake::FakeClock;
use swtos_frontend::protocol::Mode;
use swtos_session::state::Session;
use swtos_session::{debugger, driver};

/// The map the browser fetches at runtime, embedded in the test binary only.
const DEBUG_MAP: &str = include_str!("../../../assets/program.debug.json");

fn settle(session: &mut Session) {
    driver::run(session, 400, f64::MAX, &FakeClock::new(0.0));
}

fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(200, 50)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn type_line(session: &mut Session, line: &str) {
    for ch in line.chars() {
        debugger::key(
            &mut session.console,
            &mut session.panes.desktop,
            &ch.to_string(),
        );
    }
    debugger::key(&mut session.console, &mut session.panes.desktop, "Enter");
}

#[test]
fn list_falls_back_to_the_line_the_map_holds() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);

    // The identity reply is what lets the map be matched against the target.
    driver::load_map(&mut session, DEBUG_MAP);
    settle(&mut session);
    let loaded = screen(&session);
    assert!(
        loaded.contains("debug map loaded"),
        "the map did not load: {loaded}"
    );

    type_line(&mut session, "list 0");
    let text = screen(&session);

    // The map names a source file and a line for every instruction, so the
    // fallback still says where the code came from.
    assert!(
        text.contains(".s:"),
        "list said nothing useful about the source: {text}"
    );
    // The failures worth ruling out: a filesystem reach, and giving up.
    assert!(
        !text.contains("cannot read"),
        "list touched a filesystem: {text}"
    );
    assert!(
        !text.contains("no source for"),
        "list gave up where the map had an answer: {text}"
    );
}
