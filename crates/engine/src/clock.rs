//! The time, as a value the application passes in (rule R3.2).

use model::Timestamp;

/// Where the engine learns the time of day, for the log. The application gives the system's clock; tests a fixed one.
pub trait Clock {
    /// The time now.
    fn now(&self) -> Timestamp;
}

/// A clock that always says the same time, for tests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FixedClock(pub Timestamp);

impl Clock for FixedClock {
    fn now(&self) -> Timestamp {
        self.0
    }
}
