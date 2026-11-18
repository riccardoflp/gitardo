//! The groups of tasks of a repository (`tasks.edit-groups`): what a group is, what the Tasks tab asks of the groups,
//! and the groups after each change.

use super::{TaskReference, Tasks};

/// The folder of the common git folder that holds what Gitardo keeps for the repository.
pub const GROUPS_FOLDER: &str = "gitardo";

/// The file of the groups of tasks in that folder, the same for every worktree of the repository.
pub const GROUPS_FILE: &str = "task-groups.json";

/// A group of tasks, made by the user.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskGroup {
    /// Its name, which no other group of the repository has.
    pub name: String,
    /// Its tasks in their order, each once at most. One the worktree shown does not have stays, for the worktrees
    /// that have it.
    pub tasks: Vec<TaskReference>,
}

/// One step up or down a list, for **Move up** and **Move down**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shift {
    /// Before the one before it.
    Up,
    /// After the one after it.
    Down,
}

/// What the Tasks tab asks of the groups.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsAction {
    /// **New group**, whose name is asked; with a task, **New group…** of **Add to group**, which puts the task in it.
    New {
        /// The task the new group starts with, when it was made for one.
        task: Option<TaskReference>,
    },
    /// **Rename** a group, whose new name is asked.
    Rename {
        /// The group.
        group: String,
    },
    /// **Delete** a group, after asking.
    Delete {
        /// The group.
        group: String,
    },
    /// A group dragged to another place.
    PlaceGroup {
        /// The group.
        group: String,
        /// The group it goes before; none puts it last.
        before: Option<String>,
    },
    /// **Move up** or **Move down** on a group.
    ShiftGroup {
        /// The group.
        group: String,
        /// Which way.
        shift: Shift,
    },
    /// A task added to a group from its menu or dragged onto it, or dragged to another place of its group.
    PlaceTask {
        /// The group it goes in.
        group: String,
        /// The task.
        task: TaskReference,
        /// The task of the group it goes before; none puts it last.
        before: Option<TaskReference>,
        /// The group it was dragged out of, which no longer has it; none for a task that stays where it was.
        from: Option<String>,
    },
    /// **Move up** or **Move down** on a task of a group.
    ShiftTask {
        /// The group.
        group: String,
        /// The task.
        task: TaskReference,
        /// Which way.
        shift: Shift,
    },
    /// A task taken out of a group.
    RemoveTask {
        /// The group.
        group: String,
        /// The task.
        task: TaskReference,
    },
}

/// What the operations on the groups of tasks are invoked on.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsTarget {
    /// A change of the groups (`tasks.edit-groups`).
    Edit(GroupsAction),
    /// **Import groups…**, with the tasks of the worktree shown as last read, which say what the names of the file
    /// stand for and which of them the worktree lacks (`tasks.import-groups`).
    Import(Tasks),
}

/// A change of the groups, once the names it needs are known.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsEdit {
    /// A new group with this name, last, with a task when it was made for one.
    Create {
        /// Its name.
        name: String,
        /// Its first task.
        task: Option<TaskReference>,
    },
    /// A group named again.
    Rename {
        /// The group.
        group: String,
        /// Its new name.
        name: String,
    },
    /// What the Tasks tab asked that needs no name: everything but a new group and a new name.
    Do(GroupsAction),
}

/// Why a change of the groups changes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroupsRefusal {
    /// The name is empty.
    NoName,
    /// Another group has the name.
    NameTaken(String),
    /// No group has the name: another window deleted or renamed it.
    NoGroup(String),
    /// The group does not have the task.
    NoTask {
        /// The group.
        group: String,
        /// The task.
        task: TaskReference,
    },
    /// The groups are already as asked: a task already where it goes, the first group moved up.
    Unchanged,
}

impl GroupsRefusal {
    /// The refusal in words.
    #[must_use]
    pub fn said(&self) -> String {
        match self {
            GroupsRefusal::NoName => String::from("Enter a name for the group."),
            GroupsRefusal::NameTaken(name) => format!("There is already a group named `{name}`."),
            GroupsRefusal::NoGroup(name) => format!("There is no group named `{name}` any more."),
            GroupsRefusal::NoTask { group, task } => format!("The group `{group}` does not have `{task}`."),
            GroupsRefusal::Unchanged => String::from("The groups are already so."),
        }
    }
}

/// The groups after a change, in their order.
///
/// # Errors
///
/// Why nothing changes: a name empty or taken, a group or a task that is not there, or groups already as asked.
pub fn edited_groups(groups: &[TaskGroup], edit: &GroupsEdit) -> Result<Vec<TaskGroup>, GroupsRefusal> {
    let mut edited = groups.to_vec();
    match edit {
        GroupsEdit::Create { name, task } => {
            let name = free_name(groups, name, None)?;
            let tasks = task.iter().cloned().collect();
            edited.push(TaskGroup { name, tasks });
        }
        GroupsEdit::Rename { group, name } => {
            let name = free_name(groups, name, Some(group))?;
            group_of(&mut edited, group)?.name = name;
        }
        GroupsEdit::Do(action) => act(&mut edited, action)?,
    }
    if edited == groups {
        return Err(GroupsRefusal::Unchanged);
    }
    Ok(edited)
}

/// A name without the spaces around it, when no group but the one being renamed has it.
fn free_name(groups: &[TaskGroup], name: &str, renamed: Option<&String>) -> Result<String, GroupsRefusal> {
    let name = name.trim();
    if name.is_empty() {
        return Err(GroupsRefusal::NoName);
    }
    let is_taken = groups
        .iter()
        .any(|group| group.name == name && Some(&group.name) != renamed);
    if is_taken {
        return Err(GroupsRefusal::NameTaken(name.to_owned()));
    }
    Ok(name.to_owned())
}

/// The group of a name.
fn group_of<'a>(groups: &'a mut [TaskGroup], name: &str) -> Result<&'a mut TaskGroup, GroupsRefusal> {
    groups
        .iter_mut()
        .find(|group| group.name == name)
        .ok_or_else(|| GroupsRefusal::NoGroup(name.to_owned()))
}

/// Where a group is in the list.
fn place_of(groups: &[TaskGroup], name: &str) -> Result<usize, GroupsRefusal> {
    groups
        .iter()
        .position(|group| group.name == name)
        .ok_or_else(|| GroupsRefusal::NoGroup(name.to_owned()))
}

/// Does what the Tasks tab asked that needs no name.
fn act(groups: &mut Vec<TaskGroup>, action: &GroupsAction) -> Result<(), GroupsRefusal> {
    match action {
        // A new group and a new name come with their names, as `GroupsEdit::Create` and `GroupsEdit::Rename`.
        GroupsAction::New { .. } | GroupsAction::Rename { .. } => Err(GroupsRefusal::NoName),
        GroupsAction::Delete { group } => {
            let place = place_of(groups, group)?;
            let _deleted = groups.remove(place);
            Ok(())
        }
        GroupsAction::PlaceGroup { group, before } => place_group(groups, group, before.as_deref()),
        GroupsAction::ShiftGroup { group, shift } => {
            let place = place_of(groups, group)?;
            shifted(groups, place, *shift);
            Ok(())
        }
        GroupsAction::PlaceTask {
            group,
            task,
            before,
            from,
        } => {
            let _checked = group_of(groups, group)?;
            if let Some(from) = from.as_ref().filter(|from| *from != group) {
                group_of(groups, from)?.tasks.retain(|other| other != task);
            }
            place_task(&mut group_of(groups, group)?.tasks, task, before.as_ref());
            Ok(())
        }
        GroupsAction::ShiftTask { group, task, shift } => {
            let tasks = &mut group_of(groups, group)?.tasks;
            let place = task_place(tasks, group, task)?;
            shifted(tasks, place, *shift);
            Ok(())
        }
        GroupsAction::RemoveTask { group, task } => {
            let tasks = &mut group_of(groups, group)?.tasks;
            let place = task_place(tasks, group, task)?;
            let _removed = tasks.remove(place);
            Ok(())
        }
    }
}

/// Puts a group before another, or last; before itself, it stays where it is.
fn place_group(groups: &mut Vec<TaskGroup>, group: &str, before: Option<&str>) -> Result<(), GroupsRefusal> {
    let from = place_of(groups, group)?;
    if before == Some(group) {
        return Ok(());
    }
    if let Some(before) = before {
        let _checked = place_of(groups, before)?;
    }
    let moved = groups.remove(from);
    let place = before
        .and_then(|before| groups.iter().position(|other| other.name == before))
        .unwrap_or(groups.len());
    groups.insert(place, moved);
    Ok(())
}

/// Puts a task in the tasks of a group before another of them, or last: one the group has already moves there, so
/// that it is in the group once.
fn place_task(tasks: &mut Vec<TaskReference>, task: &TaskReference, before: Option<&TaskReference>) {
    let is_there = tasks.contains(task);
    if is_there && before == Some(task) {
        return;
    }
    if is_there && before.is_none() {
        // Added again from its menu, it stays where it is rather than going last.
        return;
    }
    tasks.retain(|other| other != task);
    let place = before
        .and_then(|before| tasks.iter().position(|other| other == before))
        .unwrap_or(tasks.len());
    tasks.insert(place, task.clone());
}

/// Where a task is in the tasks of a group.
fn task_place(tasks: &[TaskReference], group: &str, task: &TaskReference) -> Result<usize, GroupsRefusal> {
    tasks
        .iter()
        .position(|other| other == task)
        .ok_or_else(|| missing(group, task))
}

/// The refusal for a task its group does not have.
fn missing(group: &str, task: &TaskReference) -> GroupsRefusal {
    GroupsRefusal::NoTask {
        group: group.to_owned(),
        task: task.clone(),
    }
}

/// Moves the item at a place one step up or down, when there is room.
fn shifted<Item>(items: &mut [Item], place: usize, shift: Shift) {
    let other = match shift {
        Shift::Up => place.checked_sub(1),
        Shift::Down => Some(place + 1).filter(|after| *after < items.len()),
    };
    if let Some(other) = other {
        items.swap(place, other);
    }
}

/// A task of a group as the Tasks tab lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupEntry {
    /// The task.
    pub task: TaskReference,
    /// Whether the worktree shown has it; one it lacks is dimmed and says so.
    pub is_here: bool,
}

impl GroupEntry {
    /// **`<task>` is not in this worktree.**, for a task the worktree shown lacks.
    #[must_use]
    pub fn missing_said(&self) -> Option<String> {
        (!self.is_here).then(|| format!("`{}` is not in this worktree.", self.task))
    }
}

/// The tasks of a group as the Tasks tab lists them, in the order of the group: those the worktree shown has, and
/// those it lacks, which stay in the group for the worktrees that have them.
#[must_use]
pub fn group_entries(group: &TaskGroup, tasks: &Tasks) -> Vec<GroupEntry> {
    group
        .tasks
        .iter()
        .map(|task| GroupEntry {
            task: task.clone(),
            is_here: tasks.find(task).is_some(),
        })
        .collect()
}

#[cfg(test)]
#[path = "groups_tests.rs"]
mod tests;
