//! How long each activity has run. The activities say when they started in whole seconds; the window counts from when
//! it first saw each, which is precise enough for the half second after which the header shows it.

use std::collections::HashMap;
use std::time::Instant;

use engine::ActivityEntry;
use views::activity::RunningTask;

/// When each activity running was first seen.
#[derive(Debug, Default)]
pub struct RunningClock {
    /// The time each activity was first seen, by its number.
    seen: HashMap<u64, Instant>,
}

impl RunningClock {
    /// What runs, for the views, with how long each has run; the activities that ended are forgotten.
    pub fn tasks(&mut self, running: &[ActivityEntry]) -> Vec<RunningTask> {
        let now = Instant::now();
        self.seen
            .retain(|number, _| running.iter().any(|entry| entry.number == *number));
        running
            .iter()
            .map(|entry| {
                let first = *self.seen.entry(entry.number).or_insert(now);
                RunningTask {
                    title: entry.description.to_string(),
                    elapsed: now.duration_since(first),
                }
            })
            .collect()
    }
}
