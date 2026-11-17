//! `tasks.read`: the tasks the worktree shown defines, by source, from the files found in it.

use model::tasks::{TASKS_FILE, TaskFileProblem, Tasks, TasksContext};

use super::packages::package_scripts;
use super::workspace::workspace_tasks;

/// The tasks of the worktree shown, as the Tasks tab and **Run Task…** list them: under **Workspace** those of
/// `.vscode/tasks.json`, in the order of the file, each with what it runs on the system at hand and its variables
/// resolved; under **npm** the scripts of every `package.json`, one group for each file, run with the package manager
/// its lock file names. A file that is not JSON lists nothing, and is among the problems said at the top of the list.
///
/// The context holds everything it reads: the files as they are on disk, which `git::read::tasks::read_task_files`
/// finds, the folder of the worktree, the system, the home folder and the environment.
///
/// Feature: tasks.read
#[must_use]
pub fn read_tasks(context: &TasksContext) -> Tasks {
    let (packages, mut problems) = package_scripts(context);
    let text = context.files.tasks_json.as_deref().unwrap_or_default();
    let workspace = match workspace_tasks(text, context, &packages) {
        Ok(tasks) => tasks,
        Err(error) => {
            let problem = TaskFileProblem {
                file: String::from(TASKS_FILE),
                why: why(&error),
            };
            problems.insert(0, problem);
            Vec::new()
        }
    };
    Tasks {
        workspace,
        packages,
        problems,
    }
}

/// Why a file could not be read as JSON, as the end of a sentence: without the capital and the full stop of the
/// sentence the reader of JSON says.
pub(super) fn why(error: &json::Error) -> String {
    let said = error.to_string();
    let said = said.trim_end_matches('.');
    let mut characters = said.chars();
    match characters.next() {
        Some(first) => first.to_lowercase().chain(characters).collect(),
        None => String::new(),
    }
}

#[cfg(test)]
#[path = "read_tests.rs"]
mod tests;
