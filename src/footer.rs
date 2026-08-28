use yew::prelude::*;

const REPOSITORY: &str = "https://github.com/sw-embed/web-sw-tos";

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
