//! Launching, ending, and launching again.
//!
//! Reported: run a program from the menu, end it, return to the Shell, and the
//! menu stops responding entirely.
//!
//! sw-tos d86eade since moved the menu's programs into the shell itself, so
//! there is no second pane to end them from: one pane, and a key ends what is
//! running in it. This drives the session's byte path directly, without the
//! input crate on top.

mod fake;

use fake::FakeClock;
use swtos_frontend::protocol::Mode;
use swtos_session::driver;
use swtos_session::state::Session;

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
    // The prompt is "# " since sw-tos 808256b: one level, everything runs
    // here. It used to ask "Choice:" as though a number were the only answer.
    assert!(before.contains("MENU"), "no menu to begin with: {before}");

    // Launch from the menu. It runs in the shell, and says how to leave.
    type_keys(&mut session, &["1", "\n"]);
    let launched = screen(&session);
    assert!(
        launched.contains("Press a key here to exit"),
        "menu choice 1 launched nothing: {launched}"
    );

    // End it where it runs. The shell is the only thing typing reaches.
    let marker = screen(&session).matches("MENU").count();
    type_keys(&mut session, &[" "]);
    settle(&mut session);

    // Back at the prompt, the menu must answer again. The evidence is the
    // second program's own output: "Counter" is no evidence at all, because
    // the menu banner says "2=Counter" whether or not anything ran.
    type_keys(&mut session, &["2", "\n"]);
    settle(&mut session);
    let after = screen(&session);
    assert!(
        after.matches("MENU").count() > marker,
        "the menu stopped answering after a program ended:\n{after}"
    );
    assert!(
        after.contains("A1"),
        "the second program never ran:\n{after}"
    );
}
