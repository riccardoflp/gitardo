//! The read of the tasks of a worktree for the Tasks tab (`tasks.read`, `tasks.edit-groups`): the files that define
//! tasks, as git lists them and as they are on disk, and the groups of the repository from its common git folder.

use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::thread;

use features::tasks::groups_file::{groups_path, parse_groups};
use features::tasks::read_tasks;
use git::Git;
use git::read::tasks::read_task_files;
use model::tasks::{TaskGroup, TaskSystem, TasksContext};

use super::super::{Deliver, common_git_folder};
use crate::store::Message;
use crate::store::tasks_tab::{TaskRunMessage, TasksReadout};

/// Reads the tasks of a worktree on a thread of its own, which tells the window what it read for a repository tab.
pub(super) fn start(deliver: &Deliver, tab: PathBuf, folder: PathBuf, serial: u64) {
    let deliver = Arc::clone(deliver);
    let told = Arc::clone(&deliver);
    let key = tab.clone();
    let reading = thread::Builder::new().name(String::from("tasks read")).spawn(move || {
        let readout = Box::new(read(&folder));
        deliver(Message::TaskRun(TaskRunMessage::Read { tab, serial, readout }));
    });
    if let Err(error) = reading {
        let readout = Box::new(TasksReadout {
            problem: Some(format!("The tasks could not be read: {error}.")),
            ..TasksReadout::default()
        });
        told(Message::TaskRun(TaskRunMessage::Read {
            tab: key,
            serial,
            readout,
        }));
    }
}

/// The tasks a worktree defines and the groups of its repository, with what kept any of them from being read.
#[must_use]
pub fn read(folder: &Path) -> TasksReadout {
    let files = match read_task_files(&Git::new(folder)) {
        Ok(files) => files,
        Err(error) => {
            return TasksReadout {
                problem: Some(format!("The tasks could not be read: {error}")),
                ..TasksReadout::default()
            };
        }
    };
    let has_tasks_file = files.tasks_json.is_some();
    let environment =
        std::env::vars_os().filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)));
    let context = TasksContext {
        folder: folder.to_path_buf(),
        system: TaskSystem::current(),
        home: settings::home_folder().unwrap_or_default(),
        environment: environment.collect(),
        files,
    };
    let (groups, problem) = match groups(folder) {
        Ok(groups) => (groups, None),
        Err(problem) => (Vec::new(), Some(problem)),
    };
    TasksReadout {
        tasks: read_tasks(&context),
        groups,
        problem,
        has_tasks_file,
    }
}

/// The groups of the repository of a worktree, from the file of its common git folder; none without the file.
fn groups(folder: &Path) -> Result<Vec<TaskGroup>, String> {
    let Some(common) = common_git_folder(folder) else {
        return Ok(Vec::new());
    };
    let path = groups_path(&common);
    let text = match fs::read_to_string(&path) {
        Ok(text) => Some(text),
        Err(error) if error.kind() == ErrorKind::NotFound => None,
        Err(error) => return Err(format!("`{}` could not be read: {error}.", path.display())),
    };
    parse_groups(text.as_deref()).map_err(|problem| problem.said())
}
