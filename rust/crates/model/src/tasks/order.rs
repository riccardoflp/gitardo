//! The order in which a task and the tasks it depends on run (`tasks.run`): which run, each once, what each waits
//! for, and why nothing runs when they cannot.

use super::{DependsOrder, NotRunnable, TaskReference, Tasks};

/// A task of a run, with the tasks it waits for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunStep {
    /// The task.
    pub task: TaskReference,
    /// The tasks that have to end with exit code 0, or to be ready for a background task, before it starts: those it
    /// depends on, and, in a `sequence`, the one before it.
    pub waits_for: Vec<TaskReference>,
}

/// What running a task runs: the tasks it depends on, then the task, each once, none before a task it waits for.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RunOrder {
    /// The tasks, those that wait for nothing first; the task asked for is the last.
    pub steps: Vec<RunStep>,
}

/// Why running a task runs nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrderRefusal {
    /// The tasks depend on each other in a loop: the tasks of the loop, the first named again at the end.
    Loop(Vec<TaskReference>),
    /// A task names in `dependsOn` a task the worktree does not have.
    Missing {
        /// The task that depends on it.
        task: TaskReference,
        /// The task it names.
        named: TaskReference,
    },
    /// The task asked for is not in the worktree.
    Unknown(TaskReference),
    /// The task, or one it depends on, is one Gitardo lists and does not run.
    NotRunnable(TaskReference, NotRunnable),
}

impl OrderRefusal {
    /// The refusal in words, which name the loop or the task that is missing.
    #[must_use]
    pub fn said(&self) -> String {
        match self {
            OrderRefusal::Loop(tasks) => {
                let named: Vec<String> = tasks.iter().map(|task| format!("`{task}`")).collect();
                format!(
                    "Nothing ran: the tasks depend on each other in a loop, {}.",
                    named.join(" → ")
                )
            }
            OrderRefusal::Missing { task, named } => {
                format!("Nothing ran: `{task}` depends on `{named}`, which is not in this worktree.")
            }
            OrderRefusal::Unknown(task) => format!("Nothing ran: `{task}` is not in this worktree."),
            OrderRefusal::NotRunnable(task, why) => format!("Nothing ran: `{task}` cannot be run here. {}", why.said()),
        }
    }
}

/// The tasks a task runs, itself last, and what each waits for. With `dependsOrder` `sequence`, each task a task
/// depends on waits for the one before it, and so does what it depends on in turn, unless something earlier already
/// runs it; otherwise they wait only for their own.
///
/// # Errors
///
/// A loop, a task named that the worktree does not have, or a task Gitardo does not run: nothing runs then.
pub fn run_order(tasks: &Tasks, task: &TaskReference) -> Result<RunOrder, OrderRefusal> {
    if tasks.find(task).is_none() {
        return Err(OrderRefusal::Unknown(task.clone()));
    }
    let mut walk = Walk {
        tasks,
        order: RunOrder::default(),
        path: Vec::new(),
    };
    walk.visit(task, None)?;
    Ok(walk.order)
}

/// The walk through the tasks a task depends on.
struct Walk<'a> {
    /// The tasks of the worktree.
    tasks: &'a Tasks,
    /// The tasks placed so far.
    order: RunOrder,
    /// The tasks being walked, from the one asked for to the one at hand, to find a loop.
    path: Vec<TaskReference>,
}

impl Walk<'_> {
    /// Places a task after those it depends on. `after` is the task before it in a sequence, which it and what it
    /// depends on wait for, unless they are placed already.
    fn visit(&mut self, reference: &TaskReference, after: Option<&TaskReference>) -> Result<(), OrderRefusal> {
        if let Some(start) = self.path.iter().position(|walked| walked == reference) {
            let mut tasks: Vec<TaskReference> = self.path.iter().skip(start).cloned().collect();
            tasks.push(reference.clone());
            return Err(OrderRefusal::Loop(tasks));
        }
        if self.order.steps.iter().any(|step| step.task == *reference) {
            return Ok(());
        }
        let Some(task) = self.tasks.find(reference) else {
            return Err(OrderRefusal::Unknown(reference.clone()));
        };
        if let Some(why) = task.not_runnable() {
            return Err(OrderRefusal::NotRunnable(reference.clone(), why));
        }
        self.path.push(reference.clone());
        let mut waits_for: Vec<TaskReference> = after.into_iter().cloned().collect();
        let mut before: Option<&TaskReference> = after;
        for named in &task.depends_on {
            if self.tasks.find(named).is_none() {
                return Err(OrderRefusal::Missing {
                    task: reference.clone(),
                    named: named.clone(),
                });
            }
            self.visit(named, before)?;
            if !waits_for.contains(named) {
                waits_for.push(named.clone());
            }
            if task.depends_order == DependsOrder::Sequence {
                before = Some(named);
            }
        }
        let _walked = self.path.pop();
        self.order.steps.push(RunStep {
            task: reference.clone(),
            waits_for,
        });
        Ok(())
    }
}

#[cfg(test)]
#[path = "order_tests.rs"]
mod tests;
