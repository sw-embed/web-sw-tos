use swtos_frontend::resource::Millis;
use swtos_input::dispatch;
use swtos_session::driver;
use swtos_session::state::{Clock, LocalTime, Session};

struct Stopped;
impl Clock for Stopped {
    fn elapsed(&self) -> Millis {
        0.0
    }
    fn local(&self) -> LocalTime {
        LocalTime::default()
    }
}
fn settle(s: &mut Session) {
    driver::run(s, 700, f64::MAX, &Stopped);
}
fn type_line(s: &mut Session, text: &str) {
    for ch in text.chars() {
        dispatch::key(s, &ch.to_string(), false);
    }
    dispatch::key(s, "Enter", false);
    settle(s);
}
fn screen(s: &Session) -> String {
    s.panes
        .desktop
        .render_grid(170, 44)
        .into_iter()
        .map(|r| r.into_iter().map(|c| c.ch).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn procs(s: &Session) -> Vec<String> {
    s.panes
        .resources
        .snapshot()
        .map(|snap| {
            snap.processes
                .values()
                .map(|p| format!("ep={} {} state={}", p.endpoint, p.name, p.state))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn diag_kill() {
    let mut session = driver::session();
    settle(&mut session);
    settle(&mut session);
    println!("boot procs: {:?}", procs(&session));

    type_line(&mut session, "bg uptime");
    settle(&mut session);
    settle(&mut session);
    println!("after bg uptime: {:?}", procs(&session));
    println!("panes: {:?}", session.panes.desktop.layout());

    type_line(&mut session, "kill 2");
    settle(&mut session);
    settle(&mut session);
    println!("after kill 2: {:?}", procs(&session));
    println!("=========== SCREEN ===========\n{}", screen(&session));
}
