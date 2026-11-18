//! The context menu of a task of the Tasks tab (`tasks.view`): **Run**, **Stop**, **Restart**, **Add to group** with
//! the groups of the repository and **New group…**, what moves a task of a group or takes it out of it
//! (`tasks.edit-groups`), **Open in tasks.json** and **Copy command**.

use model::tasks::{GroupsAction, Shift};
use ui::{MenuEntry, MenuItem, Response, Ui};

use super::{TaskList, TaskRow, TasksIntent, tasks};
use crate::icons;
use crate::intent::Intent;
use crate::text::tasks_tab as words;

/// What the menu of a task does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskCommand {
    /// **Run**.
    Run,
    /// **Stop**.
    Stop,
    /// **Restart**.
    Restart,
    /// A group of **Add to group**, by its place among the groups.
    AddTo(usize),
    /// **New group…** of **Add to group**.
    NewGroup,
    /// **Move up** or **Move down**, in the group the row is in.
    Shift(Shift),
    /// **Remove from group**.
    Remove,
    /// **Open in tasks.json**.
    OpenAt,
    /// **Copy command**.
    Copy,
}

/// An item, dimmed when what it does cannot be done now.
fn able(item: MenuItem<'static, TaskCommand>, is_able: bool) -> MenuEntry<'static, TaskCommand> {
    MenuEntry::Item(if is_able { item } else { item.disabled() })
}

/// What moves the task of a row of a group, or takes it out of the group.
fn group_entries(entries: &mut Vec<MenuEntry<'_, TaskCommand>>) {
    entries.extend([
        MenuEntry::Item(icons::action(
            icons::MOVE_UP,
            words::MOVE_UP,
            TaskCommand::Shift(Shift::Up),
        )),
        MenuEntry::Item(icons::action(
            icons::MOVE_DOWN,
            words::MOVE_DOWN,
            TaskCommand::Shift(Shift::Down),
        )),
        MenuEntry::Item(icons::action(
            icons::DELETE,
            words::REMOVE_FROM_GROUP,
            TaskCommand::Remove,
        )),
        MenuEntry::Separator,
    ]);
}

/// The change of the groups an item of the menu asks for a row.
fn change(list: &TaskList, row: &TaskRow, command: TaskCommand) -> Option<GroupsAction> {
    let task = row.reference.clone();
    let group = row.group.clone();
    match command {
        TaskCommand::AddTo(place) => Some(GroupsAction::PlaceTask {
            group: list.groups.get(place)?.clone(),
            task,
            before: None,
            from: None,
        }),
        TaskCommand::NewGroup => Some(GroupsAction::New { task: Some(task) }),
        TaskCommand::Shift(shift) => Some(GroupsAction::ShiftTask {
            group: group?,
            task,
            shift,
        }),
        TaskCommand::Remove => Some(GroupsAction::RemoveTask { group: group?, task }),
        TaskCommand::Run | TaskCommand::Stop | TaskCommand::Restart | TaskCommand::OpenAt | TaskCommand::Copy => None,
    }
}

/// The context menu of a task, opened by a right click on its row.
pub(super) fn task_menu(
    list: &TaskList,
    ui: &mut Ui<'_>,
    response: &Response,
    (index, row, can_run): (usize, &TaskRow, bool),
) -> Option<Intent> {
    let is_running = row.status.is_running();
    let mut add_to: Vec<MenuEntry<'_, TaskCommand>> = Vec::new();
    for (place, name) in list.groups.iter().enumerate() {
        add_to.push(MenuEntry::Item(icons::without_icon(name, TaskCommand::AddTo(place))));
    }
    if !add_to.is_empty() {
        add_to.push(MenuEntry::Separator);
    }
    add_to.push(MenuEntry::Item(icons::action(
        icons::ADD,
        words::NEW_GROUP_FOR,
        TaskCommand::NewGroup,
    )));
    let mut entries = vec![
        able(icons::without_icon(words::RUN, TaskCommand::Run), can_run),
        able(icons::action(icons::STOP, words::STOP, TaskCommand::Stop), is_running),
        able(
            icons::without_icon(words::RESTART, TaskCommand::Restart),
            is_running && can_run,
        ),
        MenuEntry::Separator,
        MenuEntry::Submenu {
            label: words::ADD_TO_GROUP,
            icon: Some(icons::ADD),
            entries: &add_to,
        },
    ];
    if row.group.is_some() {
        group_entries(&mut entries);
    } else {
        entries.push(MenuEntry::Separator);
    }
    if row.line.is_some() {
        entries.push(MenuEntry::Item(icons::action(
            icons::OPEN_FILE,
            words::OPEN_IN_TASKS_FILE,
            TaskCommand::OpenAt,
        )));
    }
    entries.push(able(
        icons::action(icons::COPY, words::COPY_COMMAND, TaskCommand::Copy),
        !row.copied.is_empty(),
    ));
    let reference = row.reference.clone();
    let asked = match ui.context_menu(("task menu", index), response, &entries)? {
        TaskCommand::Run => TasksIntent::Run(reference),
        TaskCommand::Stop => TasksIntent::Stop(reference),
        TaskCommand::Restart => TasksIntent::Restart(reference),
        TaskCommand::OpenAt => TasksIntent::OpenAt(reference),
        TaskCommand::Copy => {
            ui.copy_to_clipboard(row.copied.clone());
            return None;
        }
        command @ (TaskCommand::AddTo(_) | TaskCommand::NewGroup | TaskCommand::Shift(_) | TaskCommand::Remove) => {
            TasksIntent::Groups(change(list, row, command)?)
        }
    };
    Some(tasks(asked))
}
