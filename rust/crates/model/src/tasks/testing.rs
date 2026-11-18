//! What the tests of the tasks share: tasks made from a few facts.

use std::path::PathBuf;

use super::{Background, DependsOrder, Task, TaskKind, TaskReference, Tasks, Word};

/// A task of `tasks.json`, by its label.
pub fn workspace(name: &str) -> TaskReference {
    TaskReference::workspace(name)
}

/// A shell task of `tasks.json` that depends on others, all at once.
pub fn task(name: &str, depends_on: &[&str]) -> Task {
    let mut task = Task::named(workspace(name), PathBuf::from("repository"));
    task.kind = TaskKind::Shell;
    task.command = Some(Word::plain(&format!("run-{name}")));
    task.depends_on = depends_on.iter().map(|name| workspace(name)).collect();
    task
}

/// The same, with those it depends on one after the other.
pub fn in_sequence(name: &str, depends_on: &[&str]) -> Task {
    let mut task = task(name, depends_on);
    task.depends_order = DependsOrder::Sequence;
    task
}

/// A background task that depends on nothing.
pub fn in_background(name: &str) -> Task {
    let mut task = task(name, &[]);
    task.background = Some(Background::default());
    task
}

/// The tasks of a worktree with only a `tasks.json`.
pub fn tasks(workspace: Vec<Task>) -> Tasks {
    Tasks {
        workspace,
        ..Tasks::default()
    }
}
