//! How long each activity has run, and what ran a moment ago. The activities say when they started in whole seconds;
//! the window counts from when it first saw each, which is precise enough for the half second after which the header
//! shows it. The moment of each frame is given, so that a test replays what the engine said at the moments it chooses.

use std::collections::{HashMap, VecDeque};
use std::time::{Duration, Instant};

use engine::ActivityEntry;
use views::activity::RunningTask;

/// How many of the things that ran the Activity tab lists.
const FINISHED_KEPT: usize = 5;
/// How long the Activity tab lists something after it ran.
const FINISHED_FOR: Duration = Duration::from_secs(30);

/// When each activity running was first seen, and what ran a moment ago.
#[derive(Debug, Default)]
pub struct RunningClock {
    /// The time each activity was first seen, by its number, with what it is.
    seen: HashMap<u64, (Instant, RunningTask)>,
    /// What ran, the latest first, with when it ended.
    finished: VecDeque<(Instant, RunningTask)>,
}

impl RunningClock {
    /// What runs at a moment, for the views, with how long each has run; the activities that ended are kept among what
    /// ran.
    pub fn tasks(&mut self, running: &[ActivityEntry], now: Instant) -> Vec<RunningTask> {
        let ended: Vec<u64> = self
            .seen
            .keys()
            .filter(|number| !running.iter().any(|entry| entry.number == **number))
            .copied()
            .collect();
        for number in ended {
            if let Some((first, mut task)) = self.seen.remove(&number) {
                task.elapsed = now.duration_since(first);
                self.finished.push_front((now, task));
            }
        }
        self.finished
            .retain(|(ended, _)| now.duration_since(*ended) < FINISHED_FOR);
        self.finished.truncate(FINISHED_KEPT);
        running
            .iter()
            .map(|entry| {
                let task = RunningTask {
                    title: entry.description.to_string(),
                    detail: Some(entry.cause.to_string()).filter(|cause| !cause.is_empty()),
                    elapsed: Duration::ZERO,
                };
                let (first, _) = self.seen.entry(entry.number).or_insert((now, task.clone()));
                RunningTask {
                    elapsed: now.duration_since(*first),
                    ..task
                }
            })
            .collect()
    }

    /// What ran a moment ago, the latest first, each with how long it took.
    #[must_use]
    pub fn finished(&self) -> Vec<RunningTask> {
        self.finished.iter().map(|(_, task)| task.clone()).collect()
    }
}
