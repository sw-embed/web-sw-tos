//! Page chrome: everything outside the character screen.
//!
//! Header and footer live together so the crate stays inside sw-checklist's
//! four-module budget while `view` stays inside its line budget.

use crate::session::Status;
use yew::prelude::*;

const REPOSITORY: &str = "https://github.com/sw-embed/web-sw-tos";

/// The two screen geometries. A terminal demo has to commit to a fixed
/// character screen, and these are the classic small and large choices.
pub const GEOMETRIES: [(usize, usize); 2] = [(80, 24), (120, 43)];

/// The line under the screen. While the prefix is armed it becomes the
/// command menu, which is the only place the Ctrl-A bindings are discoverable
/// without already knowing them.
pub fn diagnostics(status: &Status, ms: f64) -> Html {
    let text = if status.prefix_armed {
        "PREFIX -- ? help   z zoom   1-9 focus   n next   y copy-mode   \
         e send-Escape-to-app   x close"
            .to_string()
    } else {
        // The transport mode is the single most useful thing here: plain
        // means only the Shell can ever receive output.
        let transport = if status.framed {
            "framed"
        } else {
            "plain (negotiating)"
        };
        format!(
            "Ctrl-A then ? for commands   \u{2022}   transport {transport}   \
             \u{2022}   tick {}   {ms:.1} ms/tick   uart-log {}",
            status.tick, status.log_entries
        )
    };
    html! { <div class="diagnostics">{ text }</div> }
}

/// Title bar and the geometry selector. The selector is page chrome rather
/// than a terminal control: the character screen itself takes no mouse input.
pub fn header(geometry: usize, on_change: Callback<Event>) -> Html {
    html! {
        <header>
            <h1>{ "SWTOS" }</h1>
            <span class="tagline">
                { "preemptive multitasking on an emulated COR24, in your browser" }
            </span>
            <select class="geometry" onchange={on_change}>
                { for GEOMETRIES.iter().enumerate().map(|(index, (cols, rows))| html! {
                    <option selected={index == geometry}>{ format!("{cols}x{rows}") }</option>
                }) }
            </select>
        </header>
    }
}

/// Build-info footer. `sw-checklist` requires a footer naming the copyright,
/// license, repository, build host, build commit, and build time.
pub fn footer() -> Html {
    html! {
        <footer>
            <span>{ "Copyright (c) 2026 Michael A Wright" }</span>
            { html! { <span class="footer-sep">{ "\u{00b7}" }</span> } }
            <span>{ "MIT License" }</span>
            { html! { <span class="footer-sep">{ "\u{00b7}" }</span> } }
            <a href={REPOSITORY} target="_blank">{ "Repository" }</a>
            { html! { <span class="footer-sep">{ "\u{00b7}" }</span> } }
            { build_info() }
        </footer>
    }
}

fn build_info() -> Html {
    html! {
        <>
            <span>{ format!("Build Host {}", env!("BUILD_HOST")) }</span>
            { html! { <span class="footer-sep">{ "\u{00b7}" }</span> } }
            <span>{ format!("Build Commit {}", env!("BUILD_SHA")) }</span>
            { html! { <span class="footer-sep">{ "\u{00b7}" }</span> } }
            <span>{ format!("Build Time {}", env!("BUILD_TIMESTAMP")) }</span>
        </>
    }
}
