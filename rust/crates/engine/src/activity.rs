//! What is running: every read, plan and request that may take longer than a frame, with what it is in words, when
//! it started and what asked for it; and what ended since it was last asked, with when, so that a list of what runs
//! misses nothing that started and ended between two of its looks (D64).

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::Instant;

use model::Timestamp;
use model::contract::Text;

/// One thing running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivityEntry {
    /// A number of its own, to tell two entries of the same words apart.
    pub number: u64,
    /// What it is: **Pushing main to origin**.
    pub description: Text,
    /// What asked for it: **Push in the toolbar**.
    pub cause: Text,
    /// When it started.
    pub started: Timestamp,
    /// The moment it started, which how long it has run is counted from.
    pub since: Instant,
    /// Whether it is a read, which a list of what runs counts on one row with the reads of the same words around it.
    pub is_read: bool,
}

/// One thing that ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndedActivity {
    /// What it was.
    pub entry: ActivityEntry,
    /// The moment it ended.
    pub ended: Instant,
}

/// What is running, and what ended since this was last asked.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ActivityReport {
    /// What is running, in the order it started.
    pub running: Vec<ActivityEntry>,
    /// What ended, in the order it ended.
    pub ended: Vec<EndedActivity>,
}

/// How many of the things that ended are kept until they are asked for; the oldest are dropped past it, so that an
/// engine nobody asks keeps no more.
const ENDED_KEPT: usize = 256;

/// The things running, shared between the threads that start them and the views that show them.
#[derive(Debug, Clone, Default)]
pub struct Activities(Arc<Mutex<ActivityList>>);

#[derive(Debug, Default)]
struct ActivityList {
    entries: Vec<ActivityEntry>,
    next_number: u64,
    /// What ended since it was last asked, the oldest first.
    ended: VecDeque<EndedActivity>,
}

impl Activities {
    /// No activity yet.
    #[must_use]
    pub fn new() -> Self {
        Activities::default()
    }

    /// Registers something that starts now; it is running until the guard is dropped.
    ///
    /// Feature: activity.track
    #[must_use = "the activity ends when the guard is dropped"]
    pub fn track(&self, description: Text, cause: Text, started: Timestamp) -> ActivityGuard {
        self.start((description, cause), started, false)
    }

    /// Registers a read that starts now, as `track` does anything else; it is running until the guard is dropped.
    #[must_use = "the activity ends when the guard is dropped"]
    pub fn track_read(&self, description: Text, cause: Text, started: Timestamp) -> ActivityGuard {
        self.start((description, cause), started, true)
    }

    fn start(&self, (description, cause): (Text, Text), started: Timestamp, is_read: bool) -> ActivityGuard {
        let mut list = self.lock();
        let number = list.next_number;
        list.next_number += 1;
        list.entries.push(ActivityEntry {
            number,
            description,
            cause,
            started,
            since: Instant::now(),
            is_read,
        });
        ActivityGuard {
            activities: self.clone(),
            number,
        }
    }

    /// What is running, in the order it started.
    #[must_use]
    pub fn running(&self) -> Vec<ActivityEntry> {
        self.lock().entries.clone()
    }

    /// What is running, and what ended since this was last asked, which is given once: both are taken at the same
    /// moment, so that nothing is in neither.
    #[must_use]
    pub fn report(&self) -> ActivityReport {
        let mut list = self.lock();
        ActivityReport {
            running: list.entries.clone(),
            ended: list.ended.drain(..).collect(),
        }
    }

    fn lock(&self) -> MutexGuard<'_, ActivityList> {
        // A thread that panicked while holding the list left it whole: entries are pushed and removed in one step.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Keeps an activity running; dropping it ends the activity.
#[derive(Debug)]
pub struct ActivityGuard {
    activities: Activities,
    number: u64,
}

impl Drop for ActivityGuard {
    fn drop(&mut self) {
        let mut list = self.activities.lock();
        let Some(index) = list.entries.iter().position(|entry| entry.number == self.number) else {
            return;
        };
        let entry = list.entries.remove(index);
        if list.ended.len() == ENDED_KEPT {
            let _oldest = list.ended.pop_front();
        }
        list.ended.push_back(EndedActivity {
            entry,
            ended: Instant::now(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::Activities;
    use model::Timestamp;
    use model::contract::Text;

    #[test]
    fn lists_what_runs_until_its_guard_is_dropped() {
        let activities = Activities::new();
        let push = activities.track(Text::from("Pushing main to origin"), Text::from("Push"), Timestamp(10));
        let fetch = activities.track(Text::from("Fetching"), Text::from("Fetch"), Timestamp(11));
        let running: Vec<String> = activities
            .running()
            .iter()
            .map(|entry| entry.description.to_string())
            .collect();
        assert_eq!(running, ["Pushing main to origin", "Fetching"]);
        drop(push);
        assert_eq!(activities.running().len(), 1);
        drop(fetch);
        assert_eq!(activities.running(), Vec::new());
    }

    #[test]
    fn reports_once_what_ended_in_the_order_it_ended_with_when() {
        let activities = Activities::new();
        let read = activities.track_read(Text::from("Reading app"), Text::from(""), Timestamp(10));
        let fetch = activities.track(Text::from("Fetching"), Text::from("Fetch"), Timestamp(11));
        drop(fetch);
        let report = activities.report();
        let running: Vec<(&str, bool)> = report
            .running
            .iter()
            .map(|entry| (entry.description.as_str(), entry.is_read))
            .collect();
        assert_eq!(running, [("Reading app", true)]);
        let [ended] = report.ended.as_slice() else {
            panic!("{:?}", report.ended);
        };
        assert_eq!(
            (ended.entry.description.as_str(), ended.entry.is_read),
            ("Fetching", false)
        );
        assert!(ended.entry.since <= ended.ended);
        drop(read);
        let report = activities.report();
        assert_eq!(report.running, Vec::new());
        assert_eq!(report.ended.len(), 1, "what ended before is not given again");
        assert_eq!(activities.report().ended, Vec::new());
    }

    #[test]
    fn keeps_the_last_256_that_ended_while_nobody_asks() {
        let activities = Activities::new();
        for number in 0..300_i64 {
            drop(activities.track(Text::from(format!("Step {number}")), Text::from(""), Timestamp(number)));
        }
        let ended = activities.report().ended;
        assert_eq!(ended.len(), 256);
        assert_eq!(
            ended.first().map(|ended| ended.entry.description.as_str()),
            Some("Step 44")
        );
    }
}
