//! The timers of an event loop: when each goes off next, and which are due. The loop sleeps until the earliest.

use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use crate::event::TimerId;

/// The shortest interval of a timer, so that a timer of 0 does not keep the loop from ever sleeping.
const SHORTEST: Duration = Duration::from_millis(1);

/// A timer that runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Timer {
    /// When it goes off next.
    due: Instant,
    /// How long between two times it goes off.
    interval: Duration,
}

/// The timers that run.
#[derive(Clone, Debug, Default)]
pub(crate) struct Timers {
    running: BTreeMap<TimerId, Timer>,
}

impl Timers {
    /// Starts a timer that goes off every `interval` from `now`, or restarts it with this interval.
    pub(crate) fn start(&mut self, timer: TimerId, interval: Duration, now: Instant) {
        let interval = interval.max(SHORTEST);
        let _ = self.running.insert(
            timer,
            Timer {
                due: now + interval,
                interval,
            },
        );
    }

    /// Stops a timer; stopping one that is not running does nothing.
    pub(crate) fn stop(&mut self, timer: TimerId) {
        let _ = self.running.remove(&timer);
    }

    /// The timers due at `now`, the earliest first, each once even when it missed several times; each goes off next
    /// an interval later, or an interval after `now` when that is already past.
    pub(crate) fn take_due(&mut self, now: Instant) -> Vec<TimerId> {
        let mut due: Vec<(Instant, TimerId)> = self
            .running
            .iter()
            .filter(|(_, timer)| timer.due <= now)
            .map(|(id, timer)| (timer.due, *id))
            .collect();
        due.sort();
        for (_, id) in &due {
            if let Some(timer) = self.running.get_mut(id) {
                let next = timer.due + timer.interval;
                timer.due = if next <= now { now + timer.interval } else { next };
            }
        }
        due.into_iter().map(|(_, id)| id).collect()
    }

    /// When the earliest timer goes off, if any runs.
    pub(crate) fn next_due(&self) -> Option<Instant> {
        self.running.values().map(|timer| timer.due).min()
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Timers;
    use crate::event::TimerId;

    fn milliseconds(count: u64) -> Duration {
        Duration::from_millis(count)
    }

    #[test]
    fn goes_off_at_each_interval_until_stopped() {
        let start = Instant::now();
        let mut timers = Timers::default();
        timers.start(TimerId(1), milliseconds(10), start);
        timers.start(TimerId(2), milliseconds(25), start);
        assert_eq!(timers.next_due(), Some(start + milliseconds(10)));
        assert_eq!(timers.take_due(start + milliseconds(5)), vec![]);
        assert_eq!(timers.take_due(start + milliseconds(10)), vec![TimerId(1)]);
        assert_eq!(timers.next_due(), Some(start + milliseconds(20)));
        assert_eq!(timers.take_due(start + milliseconds(26)), vec![TimerId(1), TimerId(2)]);
        timers.stop(TimerId(1));
        timers.stop(TimerId(7));
        assert_eq!(timers.next_due(), Some(start + milliseconds(50)));
        timers.stop(TimerId(2));
        assert_eq!(timers.next_due(), None);
    }

    #[test]
    fn a_timer_that_missed_goes_off_once_and_catches_up() {
        let start = Instant::now();
        let mut timers = Timers::default();
        timers.start(TimerId(1), milliseconds(10), start);
        assert_eq!(timers.take_due(start + milliseconds(55)), vec![TimerId(1)]);
        assert_eq!(timers.next_due(), Some(start + milliseconds(65)));
        timers.start(TimerId(1), Duration::ZERO, start);
        assert_eq!(timers.next_due(), Some(start + milliseconds(1)));
    }
}
