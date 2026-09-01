//! Page chrome: everything outside the character screen.
//!
//! Header and footer live together so the crate stays inside sw-checklist's
//! four-module budget while `view` stays inside its line budget.

use swtos_session::state::Status;
use yew::prelude::*;

const REPOSITORY: &str = "https://github.com/sw-embed/web-sw-tos";

/// Screen geometries. `fit` is first and is the default: a terminal that
/// occupies a tenth of the window looks unfinished, and with sixteen process
/// slots the extra rows are the difference between panes you can read and
/// panes two lines tall. The fixed sizes stay for reproducing a specific
/// screen. A zero pair means "measure the window".
pub const GEOMETRIES: [(&str, usize, usize); 3] =
    [("fit", 0, 0), ("80x24", 80, 24), ("120x43", 120, 43)];

/// Size of one character cell in pixels.
///
/// Measured with a probe rather than derived from `font-size`: the advance
/// width of a monospace face is font-specific, and the stack here falls back
/// through several families, so the ratio is not knowable in advance.
fn cell_size() -> Option<(f64, f64)> {
    let document = gloo::utils::document();
    let probe = document.create_element("span").ok()?;
    probe
        .set_attribute(
            "style",
            "position:absolute;visibility:hidden;white-space:pre;\
             font-family:var(--mono);font-size:15px;line-height:1.2",
        )
        .ok()?;
    probe.set_text_content(Some(&"0".repeat(100)));
    document.body()?.append_child(&probe).ok()?;
    let rect = probe.get_bounding_client_rect();
    let size = (rect.width() / 100.0, rect.height());
    probe.remove();
    (size.0 > 0.0 && size.1 > 0.0).then_some(size)
}

/// Columns and rows that fill the stage, with a margin so the grid never
/// overflows and forces a scrollbar.
pub fn fit() -> (usize, usize) {
    let fallback = (GEOMETRIES[1].1, GEOMETRIES[1].2);
    let Some((cell_w, cell_h)) = cell_size() else {
        return fallback;
    };
    let document = gloo::utils::document();
    let Some(stage) = document.query_selector(".stage").ok().flatten() else {
        return fallback;
    };
    let rect = stage.get_bounding_client_rect();
    // The stage carries 20px of padding and the grid 10px, on each side.
    let cols = ((rect.width() - 62.0) / cell_w).floor() as usize;
    let rows = ((rect.height() - 62.0) / cell_h).floor() as usize;
    (cols.clamp(40, 400), rows.clamp(12, 200))
}

/// The line under the screen. While the prefix is armed it becomes the
/// command menu, which is the only place the Ctrl-A bindings are discoverable
/// without already knowing them.
pub fn diagnostics(status: &Status, ms: f64) -> Html {
    let text = if status.prefix_armed {
        "PREFIX -- ? help   1-9 focus   n next   z zoom   x close   \
         l clear   c close-ended   k restart-shell   B warm-reboot   \
         y copy-mode   e send-Escape-to-app"
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
                { for GEOMETRIES.iter().enumerate().map(|(index, (label, _, _))| html! {
                    <option selected={index == geometry}>{ *label }</option>
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
