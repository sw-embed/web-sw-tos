//! Page chrome: everything outside the character screen.
//!
//! Header and footer live together so the crate stays inside sw-checklist's
//! four-module budget while `view` stays inside its line budget.

use yew::prelude::*;

const REPOSITORY: &str = "https://github.com/sw-embed/web-sw-tos";

/// The two screen geometries. A terminal demo has to commit to a fixed
/// character screen, and these are the classic small and large choices.
pub const GEOMETRIES: [(usize, usize); 2] = [(80, 24), (120, 43)];

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
            { separator() }
            <span>{ "MIT License" }</span>
            { separator() }
            <a href={REPOSITORY} target="_blank">{ "Repository" }</a>
            { separator() }
            { build_info() }
        </footer>
    }
}

fn separator() -> Html {
    html! { <span class="footer-sep">{ "\u{00b7}" }</span> }
}

fn build_info() -> Html {
    html! {
        <>
            <span>{ format!("Build Host {}", env!("BUILD_HOST")) }</span>
            { separator() }
            <span>{ format!("Build Commit {}", env!("BUILD_SHA")) }</span>
            { separator() }
            <span>{ format!("Build Time {}", env!("BUILD_TIMESTAMP")) }</span>
        </>
    }
}
