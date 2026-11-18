//! The groups of the Tasks tab (`tasks.edit-groups`, `tasks.run-group`): the heading **Groups** with **New group** and
//! the menu with **Import groups…** and **Export groups…** (`tasks.import-groups`, `tasks.export-groups`), the heading
//! of each group with **Run all** or **Stop all** and its menu, and what is dragged in the column: a task
//! onto a group or to a place in one, a group to another place.

use model::tasks::{GroupsAction, Shift, TaskReference, TaskSource};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, Key, Label, MenuEntry, Modifiers, NamedKey, Rect, Response, Tone, Ui,
};

use super::{HeadingKind, HeadingRow, TaskList, TasksIntent, tasks};
use crate::icons;
use crate::intent::Intent;
use crate::text::tasks_tab as words;

/// What parts the words of what is dragged: a character no name of a task or of a group has.
const SEPARATOR: char = '\u{1f}';

/// What is dragged in the column of the tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Dragged {
    /// A task, with the group its row is in.
    Task {
        /// The task.
        task: TaskReference,
        /// The group it is dragged out of.
        from: Option<String>,
    },
    /// A group, by its name.
    Group(String),
}

impl Dragged {
    /// What the places it is dropped on read back.
    pub(super) fn payload(&self) -> String {
        match self {
            Dragged::Group(name) => format!("task group{SEPARATOR}{name}"),
            Dragged::Task { task, from } => {
                let (source, folder) = match &task.source {
                    TaskSource::Workspace => ("workspace", ""),
                    TaskSource::Npm(folder) => ("npm", folder.as_str()),
                };
                let from = from.as_deref().unwrap_or_default();
                [source, folder, &task.name, from]
                    .iter()
                    .fold(String::from("task"), |mut said, part| {
                        said.push(SEPARATOR);
                        said.push_str(part);
                        said
                    })
            }
        }
    }

    /// What is dragged, from what its source said; none for what another view drags.
    pub(super) fn parse(payload: &str) -> Option<Dragged> {
        let parts: Vec<&str> = payload.split(SEPARATOR).collect();
        match parts.as_slice() {
            ["task group", name] => Some(Dragged::Group((*name).to_owned())),
            ["task", source, folder, name, from] => {
                let task = match *source {
                    "workspace" => TaskReference::workspace(name),
                    "npm" => TaskReference::npm(folder, name),
                    _ => return None,
                };
                let from = (!from.is_empty()).then(|| (*from).to_owned());
                Some(Dragged::Task { task, from })
            }
            _ => None,
        }
    }
}

/// A place a task or a group dragged may be dropped on, lit while one it takes is over it: the change of the groups
/// the drop asks, in the frame it is dropped.
pub(super) fn drop_place(
    ui: &mut Ui<'_>,
    rect: Rect,
    takes: impl Fn(Dragged) -> Option<GroupsAction>,
) -> Option<Intent> {
    let dragged = ui.drag_payload().as_deref().and_then(Dragged::parse)?;
    let action = takes(dragged)?;
    let dropped = ui.drop_target(rect).dropped;
    dropped.map(|_source| tasks(TasksIntent::Groups(action)))
}

/// What the menu of the heading **Groups** does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ExchangeCommand {
    /// **Import groups…**.
    Import,
    /// **Export groups…**.
    Export,
}

/// The heading **Groups**: it folds the groups as the heading of a source folds its tasks, has at its right **New
/// group** and the menu with **Import groups…** and **Export groups…**, and a group dropped on it goes last.
pub(super) fn groups_heading(
    ui: &mut Ui<'_>,
    rect: Rect,
    (index, heading): (usize, &HeadingRow),
    intents: &mut Vec<Intent>,
) {
    intents.extend(drop_place(ui, rect, |dragged| match dragged {
        Dragged::Group(group) => Some(GroupsAction::PlaceGroup { group, before: None }),
        Dragged::Task { .. } => None,
    }));
    let mut line = Area::with_gap(rect, ui.theme().sizes.spacing.small);
    let more = Button::icon(icons::MORE, words::MORE_OF_GROUPS).kind(ButtonKind::Icon);
    let new = Button::icon(icons::ADD, words::NEW_GROUP).kind(ButtonKind::Icon);
    let (more_cell, new_cell) = (line.right(rect.height), line.right(rect.height));
    super::list::heading_row(ui, line.rest(), (index, heading), intents);
    if ui.button("new group of tasks", new_cell, &new).clicked {
        intents.push(tasks(TasksIntent::Groups(GroupsAction::New { task: None })));
    }
    let entries = [
        MenuEntry::Item(icons::without_icon(words::IMPORT_GROUPS, ExchangeCommand::Import)),
        MenuEntry::Item(icons::without_icon(words::EXPORT_GROUPS, ExchangeCommand::Export)),
    ];
    let chosen = ui.menu_button("menu of the groups", more_cell, &more, &entries);
    intents.extend(chosen.map(|command| match command {
        ExchangeCommand::Import => tasks(TasksIntent::ImportGroups),
        ExchangeCommand::Export => tasks(TasksIntent::ExportGroups),
    }));
}

/// What the menu of a group does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GroupCommand {
    /// **Run all**.
    Run,
    /// **Stop all**.
    Stop,
    /// **Rename**.
    Rename,
    /// **Delete**.
    Delete,
    /// **Move up** or **Move down**.
    Shift(Shift),
}

/// What an item of the menu of a group asks.
fn asked(name: &str, command: GroupCommand) -> TasksIntent {
    let group = name.to_owned();
    match command {
        GroupCommand::Run => TasksIntent::RunGroup(group),
        GroupCommand::Stop => TasksIntent::StopGroup(group),
        GroupCommand::Rename => TasksIntent::Groups(GroupsAction::Rename { group }),
        GroupCommand::Delete => TasksIntent::Groups(GroupsAction::Delete { group }),
        GroupCommand::Shift(shift) => TasksIntent::Groups(GroupsAction::ShiftGroup { group, shift }),
    }
}

/// The context menu of a group: **Run all**, **Stop all**, **Rename**, **Delete**, **Move up** and **Move down**.
fn group_menu(
    ui: &mut Ui<'_>,
    response: &Response,
    (index, can_run, is_running): (usize, bool, bool),
) -> Option<GroupCommand> {
    let able = |item: ui::MenuItem<'static, GroupCommand>, is_able: bool| {
        MenuEntry::Item(if is_able { item } else { item.disabled() })
    };
    let entries = [
        able(icons::without_icon(words::RUN_ALL, GroupCommand::Run), can_run),
        able(
            icons::action(icons::STOP, words::STOP_ALL, GroupCommand::Stop),
            is_running,
        ),
        MenuEntry::Separator,
        MenuEntry::Item(icons::action(icons::RENAME, words::RENAME, GroupCommand::Rename)),
        MenuEntry::Item(icons::action(icons::DELETE, words::DELETE, GroupCommand::Delete)),
        MenuEntry::Separator,
        MenuEntry::Item(icons::action(
            icons::MOVE_UP,
            words::MOVE_UP,
            GroupCommand::Shift(Shift::Up),
        )),
        MenuEntry::Item(icons::action(
            icons::MOVE_DOWN,
            words::MOVE_DOWN,
            GroupCommand::Shift(Shift::Down),
        )),
    ];
    ui.context_menu(("task group menu", index), response, &entries)
}

/// What a task or a group dropped on the heading of a group asks: a task of a source or of another group goes last in
/// the group, a group before it.
fn dropped_on_group(ui: &mut Ui<'_>, rect: Rect, name: &str) -> Option<Intent> {
    drop_place(ui, rect, |dragged| match dragged {
        // A task of the group itself has its place in it already.
        Dragged::Task { from: Some(from), .. } if from == name => None,
        Dragged::Task { task, from } => Some(GroupsAction::PlaceTask {
            group: name.to_owned(),
            task,
            before: None,
            from,
        }),
        Dragged::Group(other) if other != name => Some(GroupsAction::PlaceGroup {
            group: other,
            before: Some(name.to_owned()),
        }),
        Dragged::Group(_) => None,
    })
}

/// **Run all**, or **Stop all** while a task of the group runs, at the right of its heading; **Run all** is dimmed in
/// a folder not trusted.
fn group_button(
    ui: &mut Ui<'_>,
    line: &mut Area,
    (index, name, is_running, can_run): (usize, &str, bool, bool),
) -> Option<Intent> {
    let words = if is_running { words::STOP_ALL } else { words::RUN_ALL };
    let mut button = Button::secondary(words).kind(ButtonKind::Icon);
    if !is_running && !can_run {
        button = button.disabled();
    }
    let (width, height) = ui.button_size(&button);
    let cell = place(line.right(width), (width, height), (Align::End, Align::Center));
    let command = if is_running {
        GroupCommand::Stop
    } else {
        GroupCommand::Run
    };
    let clicked = ui.button(("run or stop the group", index), cell, &button).clicked;
    clicked.then(|| tasks(asked(name, command)))
}

/// The heading of a group: its chevron, a spinner while one of its tasks runs, its name, and **Run all**, or **Stop
/// all** while one runs, at its right. A click folds or unfolds its tasks and marks it as the group `F2` renames, a
/// right click opens its menu; it is dragged to another place among the groups, and a task dropped on it goes last in
/// it, a group before it.
pub(super) fn group_heading(
    list: &TaskList,
    ui: &mut Ui<'_>,
    rect: Rect,
    (index, heading): (usize, &HeadingRow),
    intents: &mut Vec<Intent>,
) {
    let HeadingKind::Group {
        name,
        is_running,
        is_selected,
    } = &heading.kind
    else {
        return;
    };
    intents.extend(dropped_on_group(ui, rect, name));
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let payload = Dragged::Group(name.clone()).payload();
    let response = ui
        .drag_source_carrying(("task group", index), rect, name, &payload)
        .response;
    if *is_selected {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_inactive_selection);
    } else if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let inside = inset(rect, Insets::symmetric(spacing.small, 0.0));
    let left = super::list::indent(ui, heading.depth);
    let mut line = Area::with_gap(
        Rect::new(inside.x + left, inside.y, (inside.width - left).max(0.0), inside.height),
        spacing.small,
    );
    intents.extend(group_button(ui, &mut line, (index, name, *is_running, list.is_trusted)));
    let icon = (theme.sizes.icon, theme.sizes.icon);
    let chevron = place(line.left(theme.sizes.icon), icon, (Align::Start, Align::Center));
    let folded = if heading.is_folded {
        icons::FOLDED
    } else {
        icons::UNFOLDED
    };
    ui.icon_in(chevron, folded, Tone::Muted);
    if *is_running {
        let cell = place(line.left(theme.sizes.icon), icon, (Align::Start, Align::Center));
        ui.spinner(cell);
    }
    let _ = ui.label(line.rest(), &Label::new(name));
    ui.describe(response.id, rect, &Description::new(AccessRole::Button, name));
    if response.clicked {
        intents.push(tasks(TasksIntent::Fold(heading.key.clone())));
    }
    if let Some(command) = group_menu(ui, &response, (index, list.is_trusted, *is_running)) {
        intents.push(tasks(asked(name, command)));
    }
}

/// `F2`, which renames the group clicked last, when no widget used the key.
pub(super) fn keys(list: &TaskList, ui: &Ui<'_>) -> Option<Intent> {
    let selected = list.rows.iter().find_map(|row| match row {
        super::Row::Heading(HeadingRow {
            kind:
                HeadingKind::Group {
                    name,
                    is_selected: true,
                    ..
                },
            ..
        }) => Some(name),
        super::Row::Heading(_) | super::Row::Task(_) => None,
    })?;
    let rename = Key::Named(NamedKey::Function(2));
    let mut keys = ui.unclaimed_keys().into_iter();
    keys.any(|key| key.key == rename && key.modifiers == Modifiers::NONE)
        .then(|| tasks(asked(selected, GroupCommand::Rename)))
}
