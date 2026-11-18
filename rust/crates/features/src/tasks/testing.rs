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

/// The files of a disk that is not there: what a feature asks to read is taken from them, and what its plan writes
/// goes to them, so that a test follows a change from the click to the file and back.
#[derive(Debug, Default)]
pub(super) struct Disk(pub std::collections::BTreeMap<PathBuf, Vec<u8>>);

/// A feature of the contract.
pub(super) type Feature = fn(&model::contract::Context, &model::contract::Answers) -> model::contract::Outcome;

impl Disk {
    /// The disk with one more file.
    pub(super) fn with(mut self, path: impl Into<PathBuf>, text: &str) -> Self {
        let _before = self.0.insert(path.into(), text.as_bytes().to_vec());
        self
    }

    /// The text of a file; none when it is not there.
    pub(super) fn text(&self, path: impl Into<PathBuf>) -> Option<String> {
        let bytes = self.0.get(&path.into())?;
        Some(String::from_utf8_lossy(bytes).into_owned())
    }

    /// What a feature comes to once every file it asks for is read from here: a question, a refusal or a plan.
    pub(super) fn asked(
        &self,
        feature: Feature,
        context: &model::contract::Context,
        mut answers: model::contract::Answers,
    ) -> model::contract::Outcome {
        use model::contract::{Answer, Outcome, Query};
        for _read in 0..8 {
            let answer = match feature(context, &answers) {
                Outcome::Read(key, Query::FileTexts { paths }) => (
                    key,
                    Answer::Files(paths.iter().map(|path| self.0.get(path).cloned()).collect()),
                ),
                Outcome::Read(key, Query::FileContents { path }) => {
                    (key, Answer::Contents(self.0.get(&path).cloned().unwrap_or_default()))
                }
                outcome => return outcome,
            };
            answers.insert(answer.0, answer.1);
        }
        panic!("the feature never stops reading");
    }

    /// Runs a plan that only writes files.
    pub(super) fn run(&mut self, plan: &model::contract::Plan) {
        for step in &plan.steps {
            match step {
                model::contract::Step::Write { path, contents } => {
                    let _before = self.0.insert(path.clone(), contents.clone());
                }
                other => panic!("a step that is no write: {other:?}"),
            }
        }
    }
}

/// The file of the groups of the repository of these tests.
pub(super) const FILE: &str = "/work/app/.git/gitardo/task-groups.json";

/// The main worktree of a repository in `/work/app`.
pub(super) fn main_worktree() -> model::state::RepositoryState {
    model::state::RepositoryState {
        folder: PathBuf::from("/work/app"),
        git_folder: PathBuf::from("/work/app/.git"),
        ..crate::testing::state()
    }
}

/// The groups on the disk, each as `name: task, task`.
pub(super) fn kept(disk: &Disk) -> Vec<String> {
    let groups = super::groups_file::parse_groups(disk.text(FILE).as_deref()).unwrap();
    let shown = |group: &model::tasks::TaskGroup| {
        let tasks: Vec<String> = group.tasks.iter().map(ToString::to_string).collect();
        format!("{}: {}", group.name, tasks.join(", "))
    };
    groups.iter().map(shown).collect()
}

/// A disk with these groups kept for the repository.
pub(super) fn disk_with(groups: &[model::tasks::TaskGroup]) -> Disk {
    Disk::default().with(FILE, &super::groups_file::groups_text(groups))
}

/// A group of tasks of `tasks.json`, by their labels.
pub(super) fn group(name: &str, tasks: &[&str]) -> model::tasks::TaskGroup {
    model::tasks::TaskGroup {
        name: name.to_owned(),
        tasks: tasks
            .iter()
            .map(|task| model::tasks::TaskReference::workspace(task))
            .collect(),
    }
}
