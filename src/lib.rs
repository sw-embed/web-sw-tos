mod footer;

use yew::prelude::*;

/// Grid geometry. Step 006 makes this selectable (80x24 / 120x43); the
/// scaffold pins the smaller of the two.
pub const COLS: usize = 80;
pub const ROWS: usize = 24;

pub struct App;

impl Component for App {
    type Message = ();
    type Properties = ();

    fn create(_ctx: &Context<Self>) -> Self {
        Self
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
                         tabindex="0">{ scaffold_screen() }</pre>
                </div>
                { footer::footer() }
            </>
        }
    }
}

/// Scaffold-only placeholder screen, replaced in step 006 by the vendored
/// `Desktop::render_grid()`. It draws a full-width box so that a misaligned
/// character cell is visible immediately rather than at integration time.
fn scaffold_screen() -> String {
    let inner = COLS - 2;
    let mut rows = vec![format!("|{}|", " ".repeat(inner)); ROWS];
    rows[0] = format!("+{}+", "-".repeat(inner));
    rows[ROWS - 1] = format!("+{}+", "-".repeat(inner));
    for (offset, text) in [
        (2, "SWTOS live demo"),
        (
            4,
            "scaffold only: emulator, virtual UART, and panes are not wired yet",
        ),
        (5, "see docs/plan.md for the phase order"),
        (
            ROWS - 4,
            "grid 80x24 . cells aligned . Ctrl-A prefix reserved",
        ),
    ] {
        rows[offset] = format!("|{:^inner$}|", text);
    }
    rows.join("\n")
}
