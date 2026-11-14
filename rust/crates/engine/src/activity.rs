//! What is running: every read, plan and request that may take longer than a frame, with what it is in words, when
//! it started and what asked for it.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

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
}

/// The things running, shared between the threads that start them and the views that show them.
#[derive(Debug, Clone, Default)]
pub struct Activities(Arc<Mutex<ActivityList>>);

#[derive(Debug, Default)]
struct ActivityList {
    entries: Vec<ActivityEntry>,
    next_number: u64,
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
        let mut list = self.lock();
        let number = list.next_number;
        list.next_number += 1;
        list.entries.push(ActivityEntry {
            number,
            description,
            cause,
            started,
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
        self.activities
            .lock()
            .entries
            .retain(|entry| entry.number != self.number);
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
}
