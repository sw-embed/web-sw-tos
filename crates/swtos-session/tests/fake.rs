//! Test doubles. Kept in test code so nothing here ships.

use std::cell::Cell;
use swtos_frontend::resource::Millis;
use swtos_session::state::{Clock, LocalTime};

/// A clock the test drives.
///
/// `elapsed` advances by a fixed step on every read, which is what lets a
/// deadline be reached deterministically instead of by sleeping.
pub struct FakeClock {
    now: Cell<Millis>,
    step: Millis,
    local: LocalTime,
}

impl FakeClock {
    /// A clock that never reaches a deadline, for tests about behaviour rather
    /// than timing.
    pub fn stopped() -> Self {
        Self {
            now: Cell::new(0.0),
            step: 0.0,
            local: LocalTime {
                hours: 13,
                minutes: 45,
                seconds: 7,
            },
        }
    }

    /// A clock that advances `step` milliseconds per reading.
    pub fn ticking(step: Millis) -> Self {
        Self {
            step,
            ..Self::stopped()
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
        self.local
    }
}
