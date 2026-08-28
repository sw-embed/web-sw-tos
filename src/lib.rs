mod footer;
mod session;

use gloo::timers::callback::Interval;
use js_sys::Date;
use session::Session;
use yew::prelude::*;

/// Grid geometry. Step 010 makes this selectable (80x24 / 120x43).
pub const COLS: usize = 80;
pub const ROWS: usize = 24;

/// SWTOS expects its scheduler heartbeat at 100 Hz.
const TICK_MS: f64 = 10.0;

/// How often the browser is asked to run us. A hidden tab is throttled to
/// roughly 1 Hz whatever we request, which is exactly why the work per
/// callback is derived from the wall clock rather than assumed.
const INTERVAL_MS: u32 = 10;

/// Ceiling on catch-up. A hidden tab is throttled to about 1 Hz, so without a
/// cap each callback would try to execute a full second of missed ticks in
/// one blocking burst and freeze the UI on return. The deliberate trade is
/// that a backgrounded tab lets emulated time fall behind the wall clock
/// rather than stuttering: at roughly 17 ms per tick this bounds one burst to
/// about a third of a second.
const MAX_CATCHUP: u32 = 20;

pub enum Msg {
    Tick,
}

pub struct App {
    session: Session,
    last: f64,
    ms_per_tick: f64,
    _ticker: Interval,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        Self {
            session: Session::default(),
            last: Date::now(),
            ms_per_tick: 0.0,
            _ticker: Interval::new(INTERVAL_MS, move || link.send_message(Msg::Tick)),
        }
    }

    fn update(&mut self, _ctx: &Context<Self>, _msg: Self::Message) -> bool {
        let now = Date::now();
        let owed = (((now - self.last) / TICK_MS) as u32).clamp(1, MAX_CATCHUP);
        let started = Date::now();
        self.session.step_many(owed);
        self.ms_per_tick = (Date::now() - started) / f64::from(owed);
        self.last = now;
        true
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {
        html! {
            <>
                <header>
                    <h1>{ "SWTOS" }</h1>
                    <span class="tagline">
                        { "preemptive multitasking on an emulated COR24, in your browser" }
                    </span>
                </header>
                <div class="stage">
                    <pre class="terminal" style={format!("--cols: {COLS}; --rows: {ROWS};")}
                         tabindex="0">{ self.screen() }</pre>
                </div>
                { footer::footer() }
            </>
        }
    }
}

impl App {
    /// The target's output plus a diagnostic line. Step 010 replaces this
    /// with the vendored pane model's cell grid.
    fn screen(&self) -> String {
        let (tick, log_entries) = self.session.stats();
        format!(
            "tick {tick}  {:.3} ms/tick  budget {TICK_MS} ms  uart-log {log_entries}\n{}",
            self.ms_per_tick,
            self.session.text()
        )
    }
}
