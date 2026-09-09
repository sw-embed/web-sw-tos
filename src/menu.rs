//! The command menu: prefix commands reachable with a mouse.
//!
//! Page chrome, not a terminal control -- the same standing as the geometry
//! selector, and the character screen still takes no mouse input.
//!
//! It exists because a prefix key can be taken away by the browser. `Ctrl-O`
//! was, on Linux and Windows, where it is Open File and never reaches the page
//! (issue #1). A key that has to survive three browsers on three platforms is
//! a key that will eventually be lost on one of them, so the commands need a
//! route that does not involve the keyboard at all.
//!
//! Every button is replayed through `dispatch::menu_command`, which presses
//! the prefix and then the key. Nothing here knows what a command does.

use crate::Msg;
use swtos_frontend::ui::Desktop;
use swtos_input::dispatch::{COMMANDS, PREFIX_LABEL};
use yew::prelude::*;

/// The button that opens the menu, and the menu itself when open.
pub fn view(desktop: &Desktop, open: bool, link: &html::Scope<crate::App>) -> Html {
    let toggle = link.callback(|_| Msg::MenuToggle);
    html! {
        <div class="menu">
            <button class="menu-button" onclick={toggle}>
                { if open { "commands \u{25b4}" } else { "commands \u{25be}" } }
            </button>
            if open {
                <div class="menu-panel">
                    { panes(desktop, link) }
                    { for ["Panes", "View", "Session"].iter().map(|group| {
                        commands(group, link)
                    }) }
                    <p class="menu-note">
                        { format!("Each is {PREFIX_LABEL} then the key shown.") }
                    </p>
                </div>
            }
        </div>
    }
}

/// Focus buttons for the panes that actually exist, named as the screen names
/// them. Built from the live layout rather than a fixed 1-9: the numbers mean
/// nothing without the pane behind them, and a menu offering nine when four
/// are open is a menu that lies five times.
fn panes(desktop: &Desktop, link: &html::Scope<crate::App>) -> Html {
    html! {
        <div class="menu-group">
            <span class="menu-label">{ "Focus" }</span>
            { for desktop.layout().into_iter().enumerate().take(9).map(|(index, (_, _, title))| {
                let key = (index + 1).to_string();
                let press = link.callback({
                    let key = key.clone();
                    move |_| Msg::MenuCommand(key.clone())
                });
                html! { <button onclick={press}>{ format!("{key} {title}") }</button> }
            }) }
        </div>
    }
}

/// One labelled row of the fixed command table.
fn commands(group: &str, link: &html::Scope<crate::App>) -> Html {
    html! {
        <div class="menu-group">
            <span class="menu-label">{ group.to_string() }</span>
            { for COMMANDS.iter().filter(|(name, _, _)| *name == group).map(|(_, key, label)| {
                let press = link.callback(move |_| Msg::MenuCommand((*key).to_string()));
                html! { <button onclick={press}>{ format!("{key}  {label}") }</button> }
            }) }
        </div>
    }
}
