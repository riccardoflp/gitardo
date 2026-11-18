//! The file groups of tasks are exported to and imported from (`tasks.import-groups`, `tasks.export-groups`): the one
//! **Export Groups...** of Run My Tasks 0.8.0 writes, in which a group names its tasks as VS Code names them and its
//! launch configurations, which Gitardo leaves out.

use json::{Member, Value};
use model::contract::{FileFilter, Text};
use model::tasks::{TaskGroup, TaskReference, Tasks};

/// The version of the file Run My Tasks 0.8.0 writes, the newest it reads, and the one Gitardo writes.
pub(super) const EXCHANGE_VERSION: u64 = 1;

/// The name the save dialog proposes for the file.
pub(super) const PROPOSED_NAME: &str = "task-groups.json";

/// A group of the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct Exported {
    /// Its name, without the spaces around it.
    pub name: String,
    /// Its tasks in their order, each by the name VS Code gives it.
    pub tasks: Vec<String>,
    /// How many launch configurations it has.
    pub launches: usize,
}

impl Exported {
    /// The tasks of the group in a worktree, each once: a name is the task of `tasks.json` with that label when the
    /// worktree has one, else the script VS Code names so, else a task of `tasks.json` the worktree lacks.
    pub fn references(&self, tasks: &Tasks) -> Vec<TaskReference> {
        let mut references = Vec::new();
        for name in &self.tasks {
            let reference = tasks
                .named_in_vs_code(name)
                .map_or_else(|| TaskReference::workspace(name), |task| task.reference.clone());
            if !references.contains(&reference) {
                references.push(reference);
            }
        }
        references
    }
}

/// Why a file gives no groups to import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ExchangeProblem {
    /// It is not JSON, or not the JSON of exported groups.
    NotGroups,
    /// It is of a version newer than the one Gitardo reads.
    Newer(u64),
}

impl ExchangeProblem {
    /// The problem in words, for a file of this name.
    pub fn said(&self, file: &str) -> String {
        match self {
            ExchangeProblem::NotGroups => format!("`{file}` is not a file of groups: nothing was imported."),
            ExchangeProblem::Newer(version) => {
                format!(
                    "`{file}` is a file of groups of version {version}, newer than Gitardo reads: nothing was imported."
                )
            }
        }
    }
}

/// The groups of an exported file, as Run My Tasks 0.8.0 reads it: an object with a numeric `version` and `groups`,
/// each with a `name` that is not empty and no other group has, `tasks` that are strings, and `launches`, when it has
/// them, that are strings too. What else the file holds is left aside.
///
/// # Errors
///
/// When the file is not such a file, or is of a newer version.
pub(super) fn parse_exchange(bytes: &[u8]) -> Result<Vec<Exported>, ExchangeProblem> {
    let text = String::from_utf8_lossy(bytes);
    let root = json::parse(text.trim_start_matches('\u{feff}')).map_err(|_| ExchangeProblem::NotGroups)?;
    let entries = root.get("groups").and_then(Value::as_array);
    let entries = entries.ok_or(ExchangeProblem::NotGroups)?;
    let version = root.get("version").and_then(Value::as_number);
    let version = version.ok_or(ExchangeProblem::NotGroups)?;
    // A version that is no whole number from zero up is of no file Run My Tasks wrote.
    let version = version.as_u64().ok_or(ExchangeProblem::NotGroups)?;
    if version > EXCHANGE_VERSION {
        return Err(ExchangeProblem::Newer(version));
    }
    let mut groups: Vec<Exported> = Vec::new();
    for entry in entries {
        let group = exported_of(entry).ok_or(ExchangeProblem::NotGroups)?;
        if groups.iter().any(|other| other.name == group.name) {
            return Err(ExchangeProblem::NotGroups);
        }
        groups.push(group);
    }
    Ok(groups)
}

/// A group of the file; none for one that is not as the file has them.
fn exported_of(entry: &Value) -> Option<Exported> {
    let strings = |value: &Value| -> Option<Vec<String>> {
        let items = value.as_array()?.iter();
        items.map(|item| item.as_str().map(str::to_owned)).collect()
    };
    let name = entry.get("name")?.as_str()?.trim();
    let tasks = strings(entry.get("tasks")?)?;
    let launches = match entry.get("launches") {
        Some(launches) => strings(launches)?.len(),
        None => 0,
    };
    (!name.is_empty()).then(|| Exported {
        name: name.to_owned(),
        tasks,
        launches,
    })
}

/// The text of the file for some groups, which Run My Tasks 0.8.0 reads: each task by the name VS Code gives it, and
/// no launch configurations.
pub(super) fn exchange_text(groups: &[TaskGroup]) -> String {
    let member = |name: &str, value: Value| Member {
        name: name.to_owned(),
        value,
    };
    let group = |group: &TaskGroup| {
        let tasks = group.tasks.iter().map(|task| Value::from(task.name_in_vs_code()));
        Value::Object(vec![
            member("name", Value::from(group.name.as_str())),
            member("tasks", Value::Array(tasks.collect())),
            member("launches", Value::Array(Vec::new())),
        ])
    };
    json::to_pretty_text(&Value::Object(vec![
        member("version", Value::from(EXCHANGE_VERSION)),
        member("groups", Value::Array(groups.iter().map(group).collect())),
    ]))
}

/// The files the dialogs of import and export show.
pub(super) fn filters() -> Vec<FileFilter> {
    vec![FileFilter {
        name: Text::from("JSON files"),
        extensions: vec![String::from("json")],
    }]
}

/// `1 group`, `2 groups`: a number of things with their name.
pub(super) fn counted(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}
