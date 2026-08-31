//! Launching, ending, and launching again.
//!
//! Reported: run a program from the menu, end it from its own pane with a
//! keypress, return to the Shell, and the menu stops responding entirely.

mod fake;

use fake::FakeClock;
use swtos_frontend::protocol::Mode;
use swtos_session::state::Session;
use swtos_session::{driver, routing};

fn framed() -> Session {
    let mut session = driver::session();
    driver::run(&mut session, 800, f64::MAX, &FakeClock::new(0.0));
    assert_eq!(session.transport.decoder.mode(), Mode::Framed);
    session
}

fn settle(session: &mut Session) {
    driver::run(session, 600, f64::MAX, &FakeClock::new(0.0));
}

fn screen(session: &Session) -> String {
    session
        .panes
        .desktop
        .render_grid(160, 60)
        .into_iter()
        .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Type at whichever pane currently has focus.
fn type_keys(session: &mut Session, keys: &[&str]) {
    for key in keys {
        swtos_session::sending::to_channel(
            session,
            session.panes.desktop.focused_channel(),
            key.as_bytes(),
        );
        settle(session);
    }
}

#[test]
fn the_menu_still_answers_after_a_program_ends() {
    let mut session = framed();
    let before = screen(&session);
    assert!(before.contains("Choice"), "no menu to begin with: {before}");

    // Launch from the menu.
    type_keys(&mut session, &["1", "\n"]);
    let launched = screen(&session);
    assert!(
        launched.contains("Hello") || launched.contains("Press"),
        "menu choice 1 launched nothing: {launched}"
    );

    // End it: the app is waiting on a key.
    let app = session
        .panes
        .desktop
        .layout()
        .into_iter()
        .find(|(kind, channel, _)| {
            *channel != routing::SHELL && *kind == swtos_frontend::ui::PaneKind::Application
        })
        .map(|(_, channel, _)| channel);
    if let Some(channel) = app {
        swtos_session::sending::to_channel(&mut session, channel, b" ");
        settle(&mut session);
    }

    // Back at the Shell, the menu must answer again.
    let marker = screen(&session).matches("Choice").count();
    type_keys(&mut session, &["2", "\n"]);
    let after = screen(&session);
    assert!(
        after.matches("Choice").count() > marker || after.contains("Counter"),
        "the menu stopped answering after a program ended:\n{after}"
    );
}
