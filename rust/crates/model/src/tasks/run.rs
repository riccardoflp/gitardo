//! A run of a task and of those it depends on, as it goes (`tasks.run`): which tasks start now, and which do not run
//! because one they wait for did not succeed.

use super::{RunOrder, RunStep, TaskEnd, TaskReference};

/// How a task of a run stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StepState {
    /// It waits for other tasks.
    Waiting,
    /// It runs, started by this run or running already when the run started.
    Running,
    /// It runs in the background and is ready: those that wait for it start.
    Ready,
    /// It ended with exit code 0.
    Succeeded,
    /// It did not succeed.
    Failed(TaskEnd),
    /// A task it waits for did not succeed, so it did not run.
    DidNotRun(Blocked),
}

/// The task that kept another from running.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Blocked {
    /// The task that did not succeed: the first cause, not a task that itself did not run because of it.
    pub by: TaskReference,
    /// How it ended.
    pub end: TaskEnd,
}

impl Blocked {
    /// **It did not run: `<task>` ended with exit code `<n>`.**
    #[must_use]
    pub fn said(&self) -> String {
        let task = &self.by;
        match &self.end {
            TaskEnd::Code(code) => format!("It did not run: `{task}` ended with exit code {code}."),
            TaskEnd::Stopped => format!("It did not run: `{task}` was stopped."),
            TaskEnd::NotStarted(said) => format!("It did not run: `{task}` could not be started: {}", said.trim()),
        }
    }
}

/// A run of a task and of those it depends on. The window starts the tasks [`Run::start`] gives, and says back when
/// one is ready or ended; the run says which start next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    /// The tasks in their order, each with how it stands.
    steps: Vec<(RunStep, StepState)>,
}

impl Run {
    /// A run of an order. The tasks running already are not run again: those that wait for them wait until the window
    /// says they are ready or ended.
    #[must_use]
    pub fn new(order: RunOrder, running: &[TaskReference]) -> Self {
        let steps = order
            .steps
            .into_iter()
            .map(|step| {
                let state = if running.contains(&step.task) {
                    StepState::Running
                } else {
                    StepState::Waiting
                };
                (step, state)
            })
            .collect();
        Run { steps }
    }

    /// The tasks to start now, in their order: those that waited and whose every task waited for has ended with exit
    /// code 0 or is ready. They count as running from here on. Tasks kept from running by one that did not succeed
    /// are marked so first.
    pub fn start(&mut self) -> Vec<TaskReference> {
        let mut started = Vec::new();
        for index in 0..self.steps.len() {
            let Some((step, StepState::Waiting)) = self.steps.get(index) else {
                continue;
            };
            let waited: Vec<&StepState> = step.waits_for.iter().filter_map(|task| self.state(task)).collect();
            let blocked = step
                .waits_for
                .iter()
                .find_map(|task| self.blocked_by(task))
                .map(StepState::DidNotRun);
            let is_free = waited
                .iter()
                .all(|state| matches!(state, StepState::Succeeded | StepState::Ready));
            let next = match blocked {
                Some(blocked) => blocked,
                None if is_free => StepState::Running,
                None => continue,
            };
            let Some((step, state)) = self.steps.get_mut(index) else {
                continue;
            };
            if next == StepState::Running {
                started.push(step.task.clone());
            }
            *state = next;
        }
        started
    }

    /// A background task of the run is ready: those that wait for it may start.
    pub fn ready(&mut self, task: &TaskReference) {
        self.set(task, &StepState::Running, StepState::Ready);
    }

    /// A task of the run ended: with exit code 0, those that wait for it may start; otherwise those that still wait
    /// for it do not run. What started once a background task was ready goes on, however that task ends later.
    pub fn ended(&mut self, task: &TaskReference, end: Result<(), TaskEnd>) {
        let next = match end {
            Ok(()) => StepState::Succeeded,
            Err(end) => StepState::Failed(end),
        };
        if self.state(task) == Some(&StepState::Ready) {
            self.set(task, &StepState::Ready, next);
        } else {
            self.set(task, &StepState::Running, next);
        }
    }

    /// How a task of the run stands; none for a task the run does not have.
    #[must_use]
    pub fn state(&self, task: &TaskReference) -> Option<&StepState> {
        self.steps
            .iter()
            .find(|(step, _)| step.task == *task)
            .map(|(_, state)| state)
    }

    /// The tasks of the run in their order, each with how it stands.
    pub fn steps(&self) -> impl Iterator<Item = (&TaskReference, &StepState)> {
        self.steps.iter().map(|(step, state)| (&step.task, state))
    }

    /// Whether nothing of the run waits or can start any more: every task ended, is ready or did not run.
    #[must_use]
    pub fn is_over(&self) -> bool {
        self.steps
            .iter()
            .all(|(_, state)| !matches!(state, StepState::Waiting | StepState::Running))
    }

    /// What kept a task that waits for `task` from running, when `task` did not succeed or did not run itself.
    fn blocked_by(&self, task: &TaskReference) -> Option<Blocked> {
        match self.state(task)? {
            StepState::Failed(end) => Some(Blocked {
                by: task.clone(),
                end: end.clone(),
            }),
            StepState::DidNotRun(blocked) => Some(blocked.clone()),
            StepState::Waiting | StepState::Running | StepState::Ready | StepState::Succeeded => None,
        }
    }

    /// Moves a task from one state to another; a task in another state stays as it is.
    fn set(&mut self, task: &TaskReference, from: &StepState, to: StepState) {
        if let Some((_, state)) = self
            .steps
            .iter_mut()
            .find(|(step, state)| step.task == *task && state == from)
        {
            *state = to;
        }
    }
}

#[cfg(test)]
#[path = "run_tests.rs"]
mod tests;
