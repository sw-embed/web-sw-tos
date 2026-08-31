//! Test doubles. Kept in test code so nothing here ships.

use std::cell::Cell;
use swtos_frontend::resource::Millis;
use swtos_session::state::{Clock, LocalTime};

/// A clock the test drives.
///
/// `elapsed` advances by `step` on every read, which is what lets a deadline
/// be reached deterministically instead of by sleeping. A step of zero never
/// reaches one, for tests about behaviour rather than timing.
///
/// One constructor rather than several named ones: a test binary that used
/// only some of them would trip dead-code, since this module is compiled into
/// each binary separately.
pub struct FakeClock {
    now: Cell<Millis>,
    step: Millis,
}

impl FakeClock {
    pub fn new(step: Millis) -> Self {
        Self {
            now: Cell::new(0.0),
            step,
        }
    }
}

impl Clock for FakeClock {
    fn elapsed(&self) -> Millis {
        let now = self.now.get();
        self.now.set(now + self.step);
        now
    }

    fn local(&self) -> LocalTime {
        LocalTime {
            hours: 13,
            minutes: 45,
            seconds: 7,
        }
    }
}
