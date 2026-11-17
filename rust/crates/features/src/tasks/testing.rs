//! What the tests of the tasks read share: a worktree, its files, and the tasks read from them.

use std::path::PathBuf;

use model::tasks::{PackageFile, Task, TaskFiles, TaskSystem, Tasks, TasksContext};

use super::read_tasks;

/// The folder of the main worktree in these tests, from the root of the disk as the folder of a worktree is.
pub(super) fn folder() -> PathBuf {
    PathBuf::from("/work/app")
}

pub(super) fn context(system: TaskSystem, tasks_json: Option<&str>) -> TasksContext {
    TasksContext {
        folder: folder(),
        system,
        home: PathBuf::from("home"),
        environment: [(String::from("PORT"), String::from("3000"))].into_iter().collect(),
        files: TaskFiles {
            tasks_json: tasks_json.map(str::to_owned),
            ..TaskFiles::default()
        },
    }
}

pub(super) fn with_packages(mut context: TasksContext, packages: &[(&str, &str)], lock_files: &[&str]) -> TasksContext {
    context.files.packages = packages
        .iter()
        .map(|(path, text)| PackageFile {
            path: (*path).to_owned(),
            text: (*text).to_owned(),
        })
        .collect();
    context.files.lock_files = lock_files.iter().map(|path| (*path).to_owned()).collect();
    context
}

pub(super) fn read(system: TaskSystem, tasks_json: &str) -> Tasks {
    read_tasks(&context(system, Some(tasks_json)))
}

/// The only task of a `tasks.json` with one, on Linux.
pub(super) fn one(entry: &str) -> Task {
    let tasks = read(
        TaskSystem::Linux,
        &format!(r#"{{ "version": "2.0.0", "tasks": [{entry}] }}"#),
    );
    assert_eq!(tasks.problems, Vec::new());
    assert_eq!(tasks.workspace.len(), 1, "{entry}");
    tasks.workspace.into_iter().next().unwrap()
}

pub(super) fn names(tasks: &[Task]) -> Vec<String> {
    tasks.iter().map(|task| task.reference.name.clone()).collect()
}

pub(super) const TWO_SHELL_TASKS: &str = r#"{
    // What the repository builds and checks.
    "version": "2.0.0",
    "tasks": [
        {
            "label": "build",
            "type": "shell",
            "command": "cargo build", /* the engine and the window */
        },
        {
            "label": "check",
            "type": "shell",
            "command": "cargo",
            "args": ["xtask", "check"],
        },
    ],
}
"#;

pub(super) const EXTENSION_PACKAGE: &str = r#"{
  "name": "gitardo",
  "scripts": {
    "build": "node esbuild.mjs",
    "test": "vitest run"
  }
}"#;
