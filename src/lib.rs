mod chrome;
pub mod session;

use gloo::events::EventListener;
use gloo::timers::callback::Timeout;
use js_sys::Date;
use session::Session;
use wasm_bindgen::JsCast;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

/// SWTOS expects its scheduler heartbeat at 100 Hz.
const TICK_MS: f64 = 10.0;

/// Wall-clock ceiling on one callback. Work is bounded by time rather than by
/// tick count because a tick's cost is not fixed, and because the browser has
/// to get the thread back on a predictable schedule whatever the emulator is
/// doing.
const BUDGET_MS: f64 = 50.0;

/// Ceiling on ticks per callback. A hidden tab is throttled to roughly 1 Hz
/// however the next tick is scheduled, so without catch-up an unfocused demo
/// would advance one tick per second. With it, a throttled tab runs what fits
/// the budget and lets emulated time fall behind rather than stealing the
/// thread -- which is the right trade for a tab nobody is looking at.
const MAX_CATCHUP: u32 = 20;

pub enum Msg {
    Tick,
    Key(String, bool),
    Geometry(usize),
}

pub struct App {
    session: Session,
    geometry: usize,
    last: f64,
    ms_per_tick: f64,
    /// The next tick, re-armed after each one completes.
    ///
    /// Deliberately not an `Interval`. A repeating timer keeps firing whether
    /// or not the previous callback has finished, and since one tick takes
    /// longer than the interval asks for, callbacks queue faster than they
    /// drain and the main thread never yields again. Self-rescheduling keeps
    /// exactly one callback outstanding, so the emulator runs as fast as it
    /// can without ever starving the page.
    next: Option<Timeout>,
    _keys: EventListener,
}

impl Component for App {
    type Message = Msg;
    type Properties = ();

    /// Keys are captured on the *window*, not on the grid. Hanging the
    /// listener off a focusable element means a stray click anywhere silently
    /// kills input; the page as a whole is the terminal. Meta and Alt
    /// combinations are left to the browser so reload, new tab, and devtools
    /// keep working; everything else, Ctrl included, belongs to the terminal.
    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let keys = ctx.link().clone();
        let listener = EventListener::new(&gloo::utils::window(), "keydown", move |event| {
            let Some(event) = event.dyn_ref::<KeyboardEvent>() else {
                return;
            };
            if event.meta_key() || event.alt_key() {
                return;
            }
            event.prevent_default();
            keys.send_message(Msg::Key(event.key(), event.ctrl_key()));
        });
        Self {
            session: Session::default(),
            geometry: 0,
            last: Date::now(),
            ms_per_tick: 0.0,
            next: Some(Timeout::new(0, move || link.send_message(Msg::Tick))),
            _keys: listener,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            Msg::Tick => {
                let started = Date::now();
                let owed = (((started - self.last) / TICK_MS) as u32).clamp(1, MAX_CATCHUP);
                self.ms_per_tick = self.session.run_until(owed, started + BUDGET_MS);
                self.last = started;
                // Aim for the 100 Hz cadence, but never schedule zero delay:
                // the browser has to get a turn between ticks.
                let delay = (TICK_MS - (Date::now() - started)).max(1.0) as u32;
                let link = ctx.link().clone();
                self.next = Some(Timeout::new(delay, move || link.send_message(Msg::Tick)));
            }
            Msg::Key(key, ctrl) => {
                self.session.send_key(&key, ctrl);
            }
            Msg::Geometry(index) => self.geometry = index.min(chrome::GEOMETRIES.len() - 1),
        }
        true
    }

    fn view(&self, ctx: &Context<Self>) -> Html {
        let (cols, rows) = chrome::GEOMETRIES[self.geometry];
        let (tick, log_entries) = self.session.stats();
        let on_geometry = ctx.link().callback(|event: Event| {
            let select: HtmlSelectElement = event.target_unchecked_into();
            Msg::Geometry(select.selected_index().max(0) as usize)
        });
        html! {
            <>
                { chrome::header(self.geometry, on_geometry) }
                <div class="stage">
                    <pre class="terminal" style={format!("--cols: {cols}; --rows: {rows};")}>
                        { self.screen(cols, rows) }
                    </pre>
                </div>
                <div class="diagnostics">
                    { format!("tick {tick}  {:.3} ms/tick  budget {TICK_MS} ms  \
                               uart-log {log_entries} entries", self.ms_per_tick) }
                </div>
                { chrome::footer() }
            </>
        }
    }
}

impl App {
    /// Flatten the pane grid into text for the `<pre>`. Cells carry colour and
    /// attributes that nothing sets yet; when they do, this is the one place
    /// that has to start emitting spans.
    fn screen(&self, cols: usize, rows: usize) -> String {
        self.session
            .grid(cols, rows)
            .into_iter()
            .map(|row| row.into_iter().map(|cell| cell.ch).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }
}
