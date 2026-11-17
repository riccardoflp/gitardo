//! The Tasks of the Activity tab (`worktree-setup.command-steps`): the commands of a new worktree, which an
//! operation hands over once its other steps ran, run apart from the operations one after the other. What the operation
//! does once it succeeded, such as opening the worktree, waits for the last of them; when one does not succeed, Gitardo
//! says which and offers to open the worktree all the same.

use std::path::PathBuf;

use engine::{LogEntry, TasksFailure};
use model::contract::{FollowUp, Step, Text};
use model::repo::ReadCause;
use model::tasks::{TaskOutput, TaskStatus, failure_said};
use views::activity::{TaskId, TaskIntent, TaskRow, TasksState};

use super::feature::{Ended, Said};
use super::{Effect, State, reads};

/// How many runs of commands the Activity tab keeps, the latest; one still running is never dropped.
const KEPT: usize = 5;

/// What a plan hands over from its first task on, with what it offers when one does not succeed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Waiting {
    /// The steps, from the first task on.
    pub steps: Vec<Step>,
    /// What the plan has the window do when a step fails: the offers to open the worktree all the same.
    pub on_failure: Vec<FollowUp>,
}

/// What the thread of a run of commands says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskMessage {
    /// A task started.
    Started(TaskId),
    /// A task printed this.
    Printed(TaskId, String),
    /// A run of commands ended: all succeeded, or what did not; and the lines it logged.
    Ended {
        /// Which run.
        batch: u64,
        /// What did not succeed; none when all did.
        failure: Option<TasksFailure>,
        /// The lines of the Command log.
        log: Vec<LogEntry>,
    },
}

/// What the store asks of the threads of the tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskEffect {
    /// Runs the steps a plan handed over, apart, as the run of this number.
    Run {
        /// Its number.
        batch: u64,
        /// The steps, from the first task on.
        steps: Vec<Step>,
    },
    /// Gives the task running in a run a line the user typed.
    Answer {
        /// Which run.
        batch: u64,
        /// The line, without its line feed.
        line: String,
    },
    /// Stops a run: the task running is stopped, and those after it do not run.
    Stop(u64),
}

/// A task, as the Activity tab shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    /// Its command.
    pub command: String,
    /// The worktree it runs in, as the views name it.
    pub name: String,
    /// The folder it runs in.
    pub folder: PathBuf,
    /// How it stands.
    pub status: TaskStatus,
    /// What it printed.
    pub output: TaskOutput,
}

/// The commands a plan handed over, with what the plan does once they all succeeded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskBatch {
    /// Its number.
    pub number: u64,
    /// The tasks, in the order they run.
    pub tasks: Vec<Task>,
    /// Whether it still runs.
    pub is_running: bool,
    /// The title of the plan.
    title: Text,
    /// What the plan does once they all succeeded.
    after: Vec<FollowUp>,
    /// Whether Undo can take the plan back.
    can_undo: bool,
    /// What the plan has the window do when one did not succeed.
    on_failure: Vec<FollowUp>,
}

/// The Tasks of the window.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct Tasks {
    /// The runs of commands, the oldest first.
    pub batches: Vec<TaskBatch>,
    /// The number of the next run.
    next: u64,
    /// The task whose output the Activity tab shows.
    pub shown: Option<TaskId>,
    /// What is typed in the box that answers the task running.
    pub typed: String,
}

impl Tasks {
    /// A task, by its number.
    #[must_use]
    pub fn task(&self, id: TaskId) -> Option<&Task> {
        self.batch(id.batch)?.tasks.get(id.index)
    }

    fn batch(&self, number: u64) -> Option<&TaskBatch> {
        self.batches.iter().find(|batch| batch.number == number)
    }

    fn batch_mut(&mut self, number: u64) -> Option<&mut TaskBatch> {
        self.batches.iter_mut().find(|batch| batch.number == number)
    }

    /// The rows of the Activity tab while it is shown, in the order the tasks run; none while it is not.
    #[must_use]
    pub fn rows_while(&self, is_shown: bool) -> Vec<TaskRow> {
        if is_shown { self.rows() } else { Vec::new() }
    }

    /// The rows of the Activity tab, in the order the tasks run.
    #[must_use]
    pub fn rows(&self) -> Vec<TaskRow> {
        let mut rows = Vec::new();
        for batch in &self.batches {
            for (index, task) in batch.tasks.iter().enumerate() {
                rows.push(TaskRow {
                    id: TaskId {
                        batch: batch.number,
                        index,
                    },
                    command: task.command.clone(),
                    name: task.name.clone(),
                    status: task.status.clone(),
                });
            }
        }
        rows
    }

    /// What the Activity tab shows of the Tasks, with its rows; none before any command ran.
    #[must_use]
    pub fn view<'a>(&'a self, rows: &'a [TaskRow]) -> Option<TasksState<'a>> {
        if rows.is_empty() {
            return None;
        }
        let output = self.shown.and_then(|shown| self.task(shown)).map(|task| &task.output);
        Some(TasksState {
            rows,
            shown: self.shown,
            output: output.map(TaskOutput::lines).unwrap_or_default(),
            dropped: output.map_or(0, TaskOutput::dropped),
            typed: &self.typed,
        })
    }
}

/// An operation ended: when its plan handed tasks over, what follows it is held until they all succeed, they start
/// in the Activity tab, which the bottom panel is shown at, and the repository is read again for the worktree made;
/// otherwise the ending is given back as it came.
///
/// # Errors
///
/// The ending, given back, when no task was handed over.
pub(super) fn hold(state: &mut State, ended: Ended) -> Result<Vec<Effect>, Box<Ended>> {
    let Ended::Ran {
        title,
        after,
        can_undo,
        waiting: Some(waiting),
        ..
    } = ended
    else {
        return Err(Box::new(ended));
    };
    let number = state.tasks.next;
    state.tasks.next += 1;
    let tasks = waiting_tasks(&waiting.steps);
    let said = after.iter().rev().find_map(|follow_up| match follow_up {
        FollowUp::Say(words) => Some(words.as_str().to_owned()),
        _ => None,
    });
    state.operations.said = said.map(|said| Said::new(said, false));
    state.tasks.batches.push(TaskBatch {
        number,
        tasks,
        is_running: true,
        title,
        after,
        can_undo,
        on_failure: waiting.on_failure,
    });
    drop_old(&mut state.tasks);
    show(
        &mut state.tasks,
        TaskId {
            batch: number,
            index: 0,
        },
    );
    let mut effects = super::panel::reduce(
        state,
        &views::panel::PanelIntent::Show(views::panel::PanelTab::Activity),
    );
    effects.extend(reads::asked(state, ReadCause::OperationEnded));
    effects.push(Effect::Tasks(TaskEffect::Run {
        batch: number,
        steps: waiting.steps,
    }));
    Ok(effects)
}

/// The tasks among the steps a plan handed over, each waiting for those before it.
fn waiting_tasks(steps: &[Step]) -> Vec<Task> {
    steps
        .iter()
        .filter_map(|step| match step {
            Step::Task {
                command,
                directory,
                name,
            } => Some(Task {
                command: command.clone(),
                name: name.clone(),
                folder: directory.clone(),
                status: TaskStatus::Waiting,
                output: TaskOutput::default(),
            }),
            _ => None, // The steps after the tasks run unseen, as the steps of any plan.
        })
        .collect()
}

/// Forgets the oldest runs that ended, beyond the [`KEPT`] latest.
fn drop_old(tasks: &mut Tasks) {
    while tasks.batches.len() > KEPT {
        let Some(oldest) = tasks.batches.iter().position(|batch| !batch.is_running) else {
            return;
        };
        let _dropped = tasks.batches.remove(oldest);
    }
}

/// Shows the output of a task, emptying the box of the answer when it is another one.
fn show(tasks: &mut Tasks, id: TaskId) {
    if tasks.shown != Some(id) {
        tasks.typed.clear();
    }
    tasks.shown = Some(id);
}

/// What the thread of a run of commands said.
pub(super) fn reduce_message(state: &mut State, message: TaskMessage) -> Vec<Effect> {
    match message {
        TaskMessage::Started(id) => {
            started(&mut state.tasks, id);
            Vec::new()
        }
        TaskMessage::Printed(id, text) => {
            if let Some(task) = task_mut(&mut state.tasks, id) {
                task.output.push(&text);
            }
            Vec::new()
        }
        TaskMessage::Ended { batch, failure, log } => {
            state.log.append(log);
            ended(state, batch, failure)
        }
    }
}

fn task_mut(tasks: &mut Tasks, id: TaskId) -> Option<&mut Task> {
    tasks.batch_mut(id.batch)?.tasks.get_mut(id.index)
}

/// A task started: the one before it succeeded, and the Activity tab follows it when it showed one of the same run.
fn started(tasks: &mut Tasks, id: TaskId) {
    let Some(batch) = tasks.batch_mut(id.batch) else {
        return;
    };
    for (index, task) in batch.tasks.iter_mut().enumerate() {
        if index < id.index && matches!(task.status, TaskStatus::Running | TaskStatus::Waiting) {
            task.status = TaskStatus::Succeeded;
        } else if index == id.index {
            task.status = TaskStatus::Running;
        }
    }
    if tasks.shown.is_some_and(|shown| shown.batch == id.batch) {
        show(tasks, id);
    }
}

/// A run of commands ended: once all succeeded, what the plan does after them is done; when a task did not succeed,
/// those after it did not run, and Gitardo says which and offers what the plan offers when it fails, such as opening
/// the worktree all the same; a step after the tasks that failed is said as the failure of a plan.
fn ended(state: &mut State, number: u64, failure: Option<TasksFailure>) -> Vec<Effect> {
    let Some(batch) = state.tasks.batch_mut(number) else {
        return Vec::new();
    };
    batch.is_running = false;
    let is_task_failure = matches!(failure, Some(TasksFailure::Task(_)));
    let mut failed = false;
    for task in &mut batch.tasks {
        task.status = match (&task.status, &failure) {
            (TaskStatus::Running | TaskStatus::Waiting, Some(TasksFailure::Task(failure))) if !failed => {
                failed = true;
                TaskStatus::Failed(failure.end.clone())
            }
            (TaskStatus::Running | TaskStatus::Waiting, _) if failed && is_task_failure => TaskStatus::DidNotRun,
            (TaskStatus::Running | TaskStatus::Waiting, _) => TaskStatus::Succeeded,
            (status, _) => status.clone(),
        };
    }
    let (title, after, can_undo) = (batch.title.clone(), batch.after.clone(), batch.can_undo);
    let on_failure = batch.on_failure.clone();
    let ending = match failure {
        None => Ended::Ran {
            title,
            after,
            can_undo,
            prepared_message: None,
            waiting: None,
        },
        Some(TasksFailure::Task(failure)) => {
            state.operations.said = Some(Said {
                runs: super::worktrees::offered(&on_failure),
                ..Said::error(failure_said(&failure))
            });
            return reads::asked(state, ReadCause::OperationEnded);
        }
        Some(TasksFailure::Step(failure)) => Ended::Failed {
            title: format!("`{}`", failure.failed),
            said: failure.said,
            plan: String::new(),
            on_failure,
        },
    };
    super::feature::ended_as_said(state, (ending, None))
}

/// What is asked of the Tasks: a task shown, an answer typed and given to the task running, a run stopped.
pub(super) fn reduce(state: &mut State, intent: TaskIntent) -> Vec<Effect> {
    let tasks = &mut state.tasks;
    match intent {
        TaskIntent::Show(id) => {
            show(tasks, id);
            Vec::new()
        }
        TaskIntent::Typed(text) => {
            tasks.typed = text;
            Vec::new()
        }
        TaskIntent::Answer => {
            let Some(id) = tasks.shown else {
                return Vec::new();
            };
            let line = std::mem::take(&mut tasks.typed);
            let Some(task) = task_mut(tasks, id).filter(|task| task.status == TaskStatus::Running) else {
                return Vec::new();
            };
            // The task reads what is typed and does not print it again, as a terminal would.
            task.output.push(&format!("{line}\n"));
            vec![Effect::Tasks(TaskEffect::Answer { batch: id.batch, line })]
        }
        TaskIntent::Stop(id) => vec![Effect::Tasks(TaskEffect::Stop(id.batch))],
    }
}
