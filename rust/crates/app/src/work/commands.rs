//! What the commands of new worktrees need besides their terminals (`worktree-setup.command-steps`): the lines the
//! Command log gets as each starts and when one does not succeed, and the steps a plan has after them, such as opening
//! the worktree in the external editor, run on a thread of their own once the commands all succeeded.

use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use engine::{Clock, CommandLog, LogLevel, TaskEvent, TaskRunner, TaskTalk, TasksFailure};
use git::{CancelToken, Git};
use model::contract::Step;

use super::Deliver;
use super::operations::SystemClock;
use crate::store::Message;
use crate::store::tasks_tab::{CommandsEffect, CommandsMessage};

/// The work of the commands of new worktrees of a window.
pub struct CommandsWork {
    deliver: Deliver,
}

impl CommandsWork {
    /// What is to come is told to `deliver`.
    #[must_use]
    pub fn new(deliver: &Deliver) -> CommandsWork {
        CommandsWork {
            deliver: std::sync::Arc::clone(deliver),
        }
    }

    /// Does what the store asks: a line of the Command log, written as `level` says, or the steps after the commands.
    pub fn take(&self, asked: CommandsEffect, level: LogLevel) {
        let mut log = CommandLog::new(level);
        match asked {
            CommandsEffect::LogStarted(step) => log.record(SystemClock.now(), &step),
            CommandsEffect::LogFailure(said) => log.record_failure(SystemClock.now(), &said),
            CommandsEffect::RunAfter { batch, folder, steps } => return self.run_after(batch, folder, steps, level),
        }
        (self.deliver)(Message::Commands(CommandsMessage::Logged(log.take())));
    }

    /// Runs the steps a plan has after its commands on a thread of its own, and tells how they ended.
    fn run_after(&self, batch: u64, folder: PathBuf, steps: Vec<Step>, level: LogLevel) {
        let deliver = std::sync::Arc::clone(&self.deliver);
        let started = thread::Builder::new()
            .name(String::from("after commands"))
            .spawn(move || {
                let mut runner = TaskRunner::new(Git::new(folder), level, Box::new(SystemClock), CancelToken::new());
                // None of these steps is a command: nothing is typed for them, and they tell nothing as they run.
                let (_input, typed) = mpsc::channel();
                let mut tell = |_event: TaskEvent| {};
                let ended = runner.run(
                    &steps,
                    &mut TaskTalk {
                        input: &typed,
                        tell: &mut tell,
                    },
                );
                deliver(Message::Commands(CommandsMessage::Ended {
                    batch,
                    failure: ended.err(),
                    log: runner.take_log(),
                }));
            });
        if let Err(error) = started {
            (self.deliver)(Message::Commands(CommandsMessage::Ended {
                batch,
                failure: Some(TasksFailure::Step(engine::StepFailure {
                    ran: Vec::new(),
                    failed: String::from("the steps after the commands"),
                    said: error.to_string(),
                    cancelled: false,
                    plan: String::new(),
                    on_failure: Box::default(),
                })),
                log: Vec::new(),
            }));
        }
    }
}
