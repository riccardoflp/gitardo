//! The file of the groups of tasks (`tasks.edit-groups`): `gitardo/task-groups.json` in the common git folder of the
//! repository, the same for every worktree, read and written whole.

use std::path::{Component, Path, PathBuf};

use json::{Member, Value};
use model::contract::{Answer, AnswerKey, Answers, Context, Outcome, Query, Refusal, Step};
use model::tasks::{GROUPS_FILE, GROUPS_FOLDER, TaskGroup, TaskReference, TaskSource};

/// The key of the `commondir` file of the git folder of the worktree shown, which names the common git folder.
pub const COMMON_FOLDER: AnswerKey = AnswerKey("common git folder");

/// The key of the file of the groups.
pub const GROUPS: AnswerKey = AnswerKey("task groups");

/// The version of the file Gitardo writes, and the newest it reads.
pub const GROUPS_VERSION: u64 = 1;

/// The `source` of a task of `.vscode/tasks.json`.
const WORKSPACE: &str = "Workspace";

/// The `source` of a script of a `package.json`.
const NPM: &str = "npm";

/// Why the file of the groups gives no groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsFileProblem {
    /// It is not JSON: what the reader of JSON says, as the end of a sentence.
    NotJson(String),
    /// It is JSON, and not the groups Gitardo writes.
    NotGroups,
    /// A newer Gitardo wrote it, with this version.
    Newer(u64),
}

impl GroupsFileProblem {
    /// The problem in words, for the top of the list of the Tasks tab.
    #[must_use]
    pub fn said(&self) -> String {
        let file = format!("{GROUPS_FOLDER}/{GROUPS_FILE}");
        match self {
            GroupsFileProblem::NotJson(why) => format!("`{file}` could not be read: {why}."),
            GroupsFileProblem::NotGroups => format!("`{file}` could not be read: it does not hold groups of tasks."),
            GroupsFileProblem::Newer(version) => {
                format!("`{file}` could not be read: a newer Gitardo wrote it, with version {version}.")
            }
        }
    }
}

/// The common git folder of a repository, from the git folder of one of its worktrees and the text of the `commondir`
/// file there: the folder the file names, from the git folder when it does not start at the root of the disk; the git
/// folder itself for the main worktree, which has no such file.
#[must_use]
pub fn common_folder(git_folder: &Path, commondir: Option<&str>) -> PathBuf {
    let named = commondir.map(str::trim).filter(|named| !named.is_empty());
    let Some(named) = named else {
        return git_folder.to_path_buf();
    };
    let mut folder = git_folder.to_path_buf();
    for part in Path::new(named).components() {
        match part {
            Component::ParentDir => {
                let _left = folder.pop();
            }
            Component::CurDir => {}
            // A prefix or a root starts again from there, which `push` does; a name goes one folder down.
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => folder.push(part),
        }
    }
    folder
}

/// The file of the groups in a common git folder.
#[must_use]
pub fn groups_path(common_folder: &Path) -> PathBuf {
    common_folder.join(GROUPS_FOLDER).join(GROUPS_FILE)
}

/// The groups of the text of the file, in its order; a repository with no file, or an empty one, has none.
///
/// # Errors
///
/// When the text is not JSON, is not what Gitardo writes, or was written by a newer Gitardo.
pub fn parse_groups(text: Option<&str>) -> Result<Vec<TaskGroup>, GroupsFileProblem> {
    let text = text.unwrap_or_default().trim_start_matches('\u{feff}');
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    let root = json::parse(text).map_err(|error| GroupsFileProblem::NotJson(super::read::why(&error)))?;
    let version = match root.get("version") {
        None => GROUPS_VERSION,
        Some(version) => version
            .as_number()
            .and_then(json::Number::as_u64)
            .ok_or(GroupsFileProblem::NotGroups)?,
    };
    if version > GROUPS_VERSION {
        return Err(GroupsFileProblem::Newer(version));
    }
    let entries = root.get("groups").and_then(Value::as_array);
    let entries = entries.ok_or(GroupsFileProblem::NotGroups)?;
    let mut groups: Vec<TaskGroup> = Vec::new();
    for entry in entries {
        let group = group_of(entry).ok_or(GroupsFileProblem::NotGroups)?;
        if groups.iter().any(|other| other.name == group.name) {
            return Err(GroupsFileProblem::NotGroups);
        }
        groups.push(group);
    }
    Ok(groups)
}

/// A group of the file: its name, and its tasks in their order, each once.
fn group_of(entry: &Value) -> Option<TaskGroup> {
    let name = entry.get("name")?.as_str()?.trim();
    let mut tasks = Vec::new();
    for task in entry.get("tasks")?.as_array()? {
        let task = task_of(task)?;
        if !tasks.contains(&task) {
            tasks.push(task);
        }
    }
    (!name.is_empty()).then(|| TaskGroup {
        name: name.to_owned(),
        tasks,
    })
}

/// A task of a group of the file, by its source and its name.
fn task_of(entry: &Value) -> Option<TaskReference> {
    let name = entry.get("name")?.as_str()?;
    match entry.get("source")?.as_str()? {
        WORKSPACE => Some(TaskReference::workspace(name)),
        NPM => Some(TaskReference::npm(entry.get("folder")?.as_str()?, name)),
        // A source Gitardo does not know is of a file it cannot keep whole.
        _ => None,
    }
}

/// The text of the file for some groups, which [`parse_groups`] reads back the same.
#[must_use]
pub fn groups_text(groups: &[TaskGroup]) -> String {
    let member = |name: &str, value: Value| Member {
        name: name.to_owned(),
        value,
    };
    let task = |task: &TaskReference| {
        let members = match &task.source {
            TaskSource::Workspace => vec![member("source", Value::from(WORKSPACE))],
            TaskSource::Npm(folder) => vec![
                member("source", Value::from(NPM)),
                member("folder", Value::from(folder.as_str())),
            ],
        };
        let name = member("name", Value::from(task.name.as_str()));
        Value::Object(members.into_iter().chain([name]).collect())
    };
    let group = |group: &TaskGroup| {
        Value::Object(vec![
            member("name", Value::from(group.name.as_str())),
            member("tasks", Value::Array(group.tasks.iter().map(task).collect())),
        ])
    };
    json::to_pretty_text(&Value::Object(vec![
        member("version", Value::from(GROUPS_VERSION)),
        member("groups", Value::Array(groups.iter().map(group).collect())),
    ]))
}

/// The groups of a repository as they are kept, and where.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Kept {
    /// The file of the groups, in the common git folder.
    pub path: PathBuf,
    /// The groups, in their order.
    pub groups: Vec<TaskGroup>,
}

impl Kept {
    /// The step that writes the groups, whole.
    pub fn write_step(&self, groups: &[TaskGroup]) -> Step {
        Step::Write {
            path: self.path.clone(),
            contents: groups_text(groups).into_bytes(),
        }
    }
}

/// The groups of the repository of the context, read from the file of its common git folder in two reads: the
/// `commondir` of the git folder of the worktree shown, then the file.
///
/// # Errors
///
/// The read still to make; the refusal outside a repository, or for a file that gives no groups, which a change must
/// not write over.
pub(super) fn kept_groups(context: &Context, answers: &Answers) -> Result<Kept, Box<Outcome>> {
    let git_folder = &context.repository.git_folder;
    if git_folder.as_os_str().is_empty() {
        let refusal = Refusal::saying("There are no groups of tasks outside a repository.");
        return Err(Box::new(Outcome::Refuse(refusal)));
    }
    let Some(Answer::Files(commondir)) = answers.get(COMMON_FOLDER) else {
        let paths = vec![git_folder.join("commondir")];
        return Err(Box::new(Outcome::Read(COMMON_FOLDER, Query::FileTexts { paths })));
    };
    let path = groups_path(&common_folder(git_folder, text_of(commondir).as_deref()));
    let Some(Answer::Files(file)) = answers.get(GROUPS) else {
        let paths = vec![path];
        return Err(Box::new(Outcome::Read(GROUPS, Query::FileTexts { paths })));
    };
    match parse_groups(text_of(file).as_deref()) {
        Ok(groups) => Ok(Kept { path, groups }),
        Err(problem) => {
            let message = format!("{} Nothing was changed.", problem.said());
            Err(Box::new(Outcome::Refuse(Refusal::saying(&message))))
        }
    }
}

/// The text of the one file a read asked for; none when it is not there.
fn text_of(files: &[Option<Vec<u8>>]) -> Option<String> {
    let bytes = files.first()?.as_ref()?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

#[cfg(test)]
#[path = "groups_file_tests.rs"]
mod tests;
