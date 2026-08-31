//! The browser's implementations of what the pure core asks for.

use crate::{App, Msg};
use gloo::events::EventListener;
use swtos_frontend::resource::Millis;
use wasm_bindgen::JsCast;
use web_sys::KeyboardEvent;
use yew::html::Scope;
use swtos_session::state::{Clock, LocalTime};

/// Time from the browser. The only implementation that touches `js_sys`;
/// tests supply their own.
pub struct BrowserClock;

impl Clock for BrowserClock {
    fn elapsed(&self) -> Millis {
        js_sys::Date::now()
    }

    fn local(&self) -> LocalTime {
        let now = js_sys::Date::new_0();
        LocalTime {
            hours: now.get_hours() as u8,
            minutes: now.get_minutes() as u8,
            seconds: now.get_seconds() as u8,
        }
    }
}

/// Window keydown, filtered to what the terminal should see.
///
/// Bound to the window rather than the grid: hanging it off a focusable
/// element means a stray click anywhere silently kills input. Meta and Alt
/// combinations are left to the browser so reload, new tab, and devtools keep
/// working; everything else, Ctrl included, belongs to the terminal.
pub fn on_keydown(link: Scope<App>) -> EventListener {
    EventListener::new(&gloo::utils::window(), "keydown", move |event| {
        let Some(event) = event.dyn_ref::<KeyboardEvent>() else {
            return;
        };
        if event.meta_key() || event.alt_key() {
            return;
        }
        event.prevent_default();
        link.send_message(Msg::Key(event.key(), event.ctrl_key()));
    })
}

/// Window resize, so the fitted geometry is recomputed.
pub fn on_resize(link: Scope<App>) -> EventListener {
    EventListener::new(&gloo::utils::window(), "resize", move |_| {
        link.send_message(Msg::Resize);
    })
}

/// Fetch the debug map, which is deliberately not compiled in: at 1.6 MB it
/// would dwarf the WASM module and be re-committed to `pages/` on every
/// rebuild. Same-origin, so the demo stays fully client-side.
pub fn fetch_debug_map(link: Scope<App>) {
    wasm_bindgen_futures::spawn_local(async move {
        if let Ok(response) = gloo::net::http::Request::get("program.debug.json").send().await
            && let Ok(text) = response.text().await
        {
            link.send_message(Msg::MapLoaded(text));
        }
    });
}
