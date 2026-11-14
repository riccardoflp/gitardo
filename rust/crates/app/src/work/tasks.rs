//! The threads of the Tasks of the Activity drawer (`worktree-setup.command-steps`): each run of commands a plan handed
//! over runs on a thread of its own, its tasks one after the other, each among the activities while it runs, and tells
//! the window what they print as it comes.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::mpsc::{self, Sender};
use std::thread;

use engine::{Activities, ActivityGuard, LogLevel, TaskEvent, TaskRunner, TaskTalk};
use git::{CancelToken, Git};
use model::contract::{Step, Text};
use views::activity::TaskId;
use views::text::activity as words;

use super::operations::SystemClock;
use super::{Deliver, now};
use crate::store::Message;
use crate::store::tasks::TaskMessage;

/// A run of commands under way: where what the user types goes, and the token that stops it.
struct Running {
    input: Sender<String>,
    cancel: CancelToken,
}

/// The runs of commands of a window.
pub struct TaskRunners {
    activities: Activities,
    deliver: Deliver,
    running: BTreeMap<u64, Running>,
}

impl TaskRunners {
    /// No run yet; those to come put their tasks among `activities` and tell `deliver` what they do.
    #[must_use]
    pub fn new(activities: &Activities, deliver: &Deliver) -> TaskRunners {
        TaskRunners {
            activities: activities.clone(),
            deliver: std::sync::Arc::clone(deliver),
            running: BTreeMap::new(),
        }
    }

    /// Runs the steps a plan handed over, as the run of this number, on a thread of its own.
    pub fn run(&mut self, batch: u64, steps: Vec<Step>, level: LogLevel) {
        let (input, typed) = mpsc::channel();
        let cancel = CancelToken::new();
        let token = cancel.clone();
        let activities = self.activities.clone();
        let deliver = std::sync::Arc::clone(&self.deliver);
        let started = thread::Builder::new().name(String::from("tasks")).spawn(move || {
            let git = Git::new(first_folder(&steps));
            let mut runner = TaskRunner::new(git, level, Box::new(SystemClock), token);
            let titles = titles(&steps);
            let mut activity: Option<ActivityGuard> = None;
            let mut tell = |event: TaskEvent| {
                let message = match event {
                    TaskEvent::Started(index) => {
                        let title = titles.get(index).cloned().unwrap_or_default();
                        activity = None;
                        activity =
                            Some(activities.track(Text::from(title), Text::from(words::RUNNING_IN_TASKS), now()));
                        TaskMessage::Started(TaskId { batch, index })
                    }
                    TaskEvent::Printed(index, text) => TaskMessage::Printed(TaskId { batch, index }, text),
                };
                deliver(Message::Task(message));
            };
            let ended = runner.run(
                &steps,
                &mut TaskTalk {
                    input: &typed,
                    tell: &mut tell,
                },
            );
            drop(activity);
            deliver(Message::Task(TaskMessage::Ended {
                batch,
                failure: ended.err(),
                log: runner.take_log(),
            }));
        });
        // A thread that could not start runs nothing: the run is said to have ended at once, which nothing waits on.
        if started.is_ok() {
            let _replaced = self.running.insert(batch, Running { input, cancel });
        }
    }

    /// Gives the task running in a run a line the user typed.
    pub fn answer(&self, batch: u64, line: String) {
        if let Some(running) = self.running.get(&batch) {
            // A run that ended reads nothing more.
            let _sent = running.input.send(line);
        }
    }

    /// Stops a run: the task running is stopped, and those after it do not run.
    pub fn stop(&self, batch: u64) {
        if let Some(running) = self.running.get(&batch) {
            running.cancel.cancel();
        }
    }

    /// Forgets a run that ended.
    pub fn finished(&mut self, batch: u64) {
        let _ended = self.running.remove(&batch);
    }
}

impl Drop for TaskRunners {
    /// The commands still running when the window closes are stopped.
    fn drop(&mut self) {
        for running in self.running.values() {
            running.cancel.cancel();
        }
    }
}

/// The folder the git of a run starts in: that of its first task; each step names its own.
fn first_folder(steps: &[Step]) -> PathBuf {
    steps
        .iter()
        .find_map(|step| match step {
            Step::Task { directory, .. } => Some(directory.clone()),
            _ => None, // Only a task begins a run.
        })
        .unwrap_or_default()
}

/// The titles of the tasks of a run among the activities, in order: `Running npm ci in fix`.
fn titles(steps: &[Step]) -> Vec<String> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Task { command, name, .. } => Some(words::running_task(command, name)),
            _ => None, // The other steps are not tasks.
        })
        .collect()
}
