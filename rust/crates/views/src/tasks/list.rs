//! The column of the tasks of the Tasks tab (`tasks.view`): the search field and the menu of the column, what the
//! column says of a folder not trusted and of the files it could not read, and the rows, each heading folding what
//! is under it.

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Description, InputBox, Label, MenuEntry, Rect, Sense, TextKind, Tone, Ui};

use super::{HeadingRow, Row, RowStatus, TaskList, TaskRow, TasksIntent, tasks};
use crate::icons;
use crate::intent::Intent;
use crate::text::tasks_tab as words;

/// What the menu of the column does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ColumnCommand {
    /// **Open tasks.json**.
    OpenFile,
    /// **Collapse all**.
    Collapse,
    /// **Expand all**.
    Expand,
}

/// What the menu of a task does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TaskCommand {
    /// **Run**.
    Run,
    /// **Stop**.
    Stop,
    /// **Restart**.
    Restart,
    /// **Open in tasks.json**.
    OpenAt,
    /// **Copy command**.
    Copy,
}

/// A line of muted words at the top of a rectangle.
pub(super) fn say(ui: &mut Ui<'_>, rect: Rect, said: &str) {
    let height = ui.theme().sizes.row_height;
    let label = Label::new(said).kind(TextKind::Detail).tone(Tone::Muted);
    let _ = ui.label(Rect::new(rect.x, rect.y, rect.width, height.min(rect.height)), &label);
}

/// The column of the tasks over the rectangle its caller clipped it to.
pub(super) fn list_view(list: &TaskList, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    let mut area = Area::with_gap(rect, spacing.small);
    title_row(list, ui, area.top(theme.sizes.control_height), &mut intents);
    if !list.is_trusted {
        not_trusted(ui, &mut area, &mut intents);
    }
    for (index, problem) in list.problems.iter().enumerate() {
        let style = (TextKind::Detail, Tone::Warning);
        let height = paragraph_height(ui, &problem.said, rect.width);
        let _ = ui.paragraph(area.top(height), &problem.said, style);
        if problem.file.is_empty() {
            continue;
        }
        let open = Button::secondary(words::OPEN_THE_FILE);
        let (width, height) = ui.button_size(&open);
        let cell = place(area.top(height), (width, height), (Align::Start, Align::Center));
        if ui.button(("open the file of tasks", index), cell, &open).clicked {
            intents.push(tasks(TasksIntent::OpenFile(problem.file.clone())));
        }
    }
    let body = area.rest();
    if !list.is_read {
        say(ui, body, words::READING);
    } else if list.rows.is_empty() {
        let said = if list.search.trim().is_empty() {
            words::NO_TASKS
        } else {
            words::NO_MATCH
        };
        let _ = ui.paragraph(body, said, (TextKind::Detail, Tone::Muted));
    } else {
        rows(list, ui, body, &mut intents);
    }
    intents
}

/// The height a paragraph of small words takes in a width.
fn paragraph_height(ui: &Ui<'_>, text: &str, width: f32) -> f32 {
    let style = ui.text_style(TextKind::Detail);
    let line = ui.line_height(style);
    let lines = (ui.measure(text, style) / width.max(1.0)).ceil().max(1.0) + 1.0;
    line * lines.min(4.0)
}

/// The top of the column: the search field, and the menu with **Open tasks.json**, **Collapse all** and **Expand
/// all**.
fn title_row(list: &TaskList, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let spacing = ui.theme().sizes.spacing;
    let mut parts = Area::with_gap(rect, spacing.small);
    let more = Button::icon(icons::MORE, words::MORE).kind(ButtonKind::Icon);
    let entries = [
        MenuEntry::Item(icons::action(
            icons::OPEN_FILE,
            words::OPEN_TASKS_FILE,
            ColumnCommand::OpenFile,
        )),
        MenuEntry::Separator,
        MenuEntry::Item(icons::without_icon(words::COLLAPSE_ALL, ColumnCommand::Collapse)),
        MenuEntry::Item(icons::without_icon(words::EXPAND_ALL, ColumnCommand::Expand)),
    ];
    match ui.menu_button("tasks column menu", parts.right(rect.height), &more, &entries) {
        Some(ColumnCommand::OpenFile) => intents.push(tasks(TasksIntent::OpenTasksFile)),
        Some(ColumnCommand::Collapse) => intents.push(tasks(TasksIntent::FoldAll(true))),
        Some(ColumnCommand::Expand) => intents.push(tasks(TasksIntent::FoldAll(false))),
        None => {}
    }
    let mut input = InputBox::new(&list.search, words::SEARCH);
    input.icon = Some(icons::SEARCH);
    if let Some(text) = ui.input_box("search tasks", parts.rest(), &input).changed_text {
        intents.push(tasks(TasksIntent::Search(text)));
    }
}

/// What the column says in a folder not trusted, with **Trust this folder**.
fn not_trusted(ui: &mut Ui<'_>, area: &mut Area, intents: &mut Vec<Intent>) {
    let width = ui.current_clip().width;
    let height = paragraph_height(ui, words::NOT_TRUSTED, width);
    let _ = ui.paragraph(area.top(height), words::NOT_TRUSTED, (TextKind::Detail, Tone::Warning));
    let trust = Button::secondary(words::TRUST);
    let (width, height) = ui.button_size(&trust);
    let cell = place(area.top(height), (width, height), (Align::Start, Align::Center));
    if ui.button("trust this folder", cell, &trust).clicked {
        intents.push(tasks(TasksIntent::Trust));
    }
}

/// The rows, scrolling when they do not fit.
fn rows(list: &TaskList, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let height = theme.sizes.row_height;
    let count = f32::from(u16::try_from(list.rows.len()).unwrap_or(u16::MAX));
    ui.region("tasks list", (rect, AccessRole::List, words::LIST), |ui| {
        let _ = ui.scroll_area("tasks rows", rect, (rect.width, height * count), |ui, inside| {
            let mut top = inside.y;
            for (index, row) in list.rows.iter().enumerate() {
                let cell = Rect::new(inside.x, top, inside.width, height);
                top += height;
                if cell.bottom() < rect.y || cell.y > rect.bottom() {
                    continue;
                }
                match row {
                    Row::Heading(heading) => heading_row(ui, cell, (index, heading), intents),
                    Row::Task(task) => task_row(list, ui, cell, (index, task), intents),
                }
            }
        });
    });
}

/// How far a row starts from the left of the column, by how far in it is.
fn indent(ui: &Ui<'_>, depth: u8) -> f32 {
    let sizes = ui.theme().sizes;
    f32::from(depth) * (sizes.icon + sizes.spacing.small)
}

/// A heading: its chevron and its name; a click folds or unfolds what is under it.
fn heading_row(ui: &mut Ui<'_>, rect: Rect, (index, heading): (usize, &HeadingRow), intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("tasks heading", index));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let inside = inset(rect, Insets::symmetric(spacing.small, 0.0));
    let mut line = Area::with_gap(
        Rect::new(
            inside.x + indent(ui, heading.depth),
            inside.y,
            (inside.width - indent(ui, heading.depth)).max(0.0),
            inside.height,
        ),
        spacing.small,
    );
    let chevron = place(
        line.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    let icon = if heading.is_folded {
        icons::FOLDED
    } else {
        icons::UNFOLDED
    };
    ui.icon_in(chevron, icon, Tone::Muted);
    let label = Label::new(&heading.name).kind(TextKind::SmallStrong).tone(Tone::Muted);
    let _ = ui.label(line.rest(), &label);
    ui.describe(id, rect, &Description::new(AccessRole::Button, &heading.name));
    if response.clicked {
        intents.push(tasks(TasksIntent::Fold(heading.key.clone())));
    }
}

/// The mark of how a task stands and what its row says of it after its name.
fn standing(status: &RowStatus) -> (Option<(ui::Icon, Tone)>, String, Tone) {
    match status {
        RowStatus::Waiting => (Some((icons::WAITS, Tone::Muted)), words::WAITS.to_owned(), Tone::Muted),
        RowStatus::Running { is_ready: true } => (None, words::READY.to_owned(), Tone::Muted),
        RowStatus::NotRun | RowStatus::Running { is_ready: false } => (None, String::new(), Tone::Muted),
        RowStatus::Succeeded => (Some((icons::CHOSEN, Tone::Success)), String::new(), Tone::Muted),
        RowStatus::Failed(code) => (Some((icons::ERROR, Tone::Error)), code.to_string(), Tone::Error),
        RowStatus::Stopped => (
            Some((icons::WARNING, Tone::Warning)),
            words::STOPPED.to_owned(),
            Tone::Muted,
        ),
        RowStatus::NotStarted => (
            Some((icons::ERROR, Tone::Error)),
            words::NOT_STARTED.to_owned(),
            Tone::Error,
        ),
        RowStatus::DidNotRun => (
            Some((icons::SKIPPED, Tone::Muted)),
            words::DID_NOT_RUN.to_owned(),
            Tone::Muted,
        ),
    }
}

/// A row of a task: the mark of how it stands, its name, its command dimmed after it when there is room, how it
/// ended, and **Run** or **Stop** at its right. A click selects it, a double click runs it or shows it when it runs, a
/// right click opens its menu.
fn task_row(list: &TaskList, ui: &mut Ui<'_>, rect: Rect, (index, row): (usize, &TaskRow), intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("task", index));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if row.is_selected {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_inactive_selection);
    } else if response.contains_pointer {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let can_run = list.is_trusted && row.cannot_run.is_none();
    let is_running = row.status.is_running();
    let inside = inset(rect, Insets::symmetric(spacing.small, 0.0));
    let left = indent(ui, row.depth);
    let mut line = Area::with_gap(
        Rect::new(inside.x + left, inside.y, (inside.width - left).max(0.0), inside.height),
        spacing.medium,
    );
    run_button(ui, &mut line, (index, row, can_run), intents);
    let (mark, said, tone) = standing(&row.status);
    let cell = place(
        line.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    match mark {
        Some((icon, tone)) => ui.icon_in(cell, icon, tone),
        None if is_running => ui.spinner(cell),
        None => {}
    }
    if !said.is_empty() {
        let width = ui.measure(&said, ui.text_style(TextKind::Detail));
        let _ = ui.label(line.right(width), &Label::new(&said).kind(TextKind::Detail).tone(tone));
    }
    let name_tone = if row.cannot_run.is_some() {
        Tone::Disabled
    } else {
        Tone::Normal
    };
    let name_width = ui
        .measure(&row.name, ui.text_style(TextKind::Body))
        .min(line.rest().width);
    let _ = ui.label(line.left(name_width), &Label::new(&row.name).tone(name_tone));
    let after = row.cannot_run.as_deref().unwrap_or(&row.command);
    let _ = ui.label(line.rest(), &Label::new(after).kind(TextKind::Detail).tone(Tone::Muted));
    let name = words::row_name(&row.name, after, &said);
    ui.describe(id, rect, &Description::new(AccessRole::ListItem, &name));
    if response.double_clicked && can_run && !is_running {
        intents.push(tasks(TasksIntent::Run(row.reference.clone())));
    } else if response.clicked || response.double_clicked {
        intents.push(tasks(TasksIntent::Select(row.reference.clone())));
    }
    intents.extend(task_menu(ui, &response, (index, row, can_run)));
}

/// **Run**, or **Stop** while the task runs, at the right of its row; **Run** is dimmed for a task that cannot run and
/// in a folder not trusted.
fn run_button(
    ui: &mut Ui<'_>,
    line: &mut Area,
    (index, row, can_run): (usize, &TaskRow, bool),
    intents: &mut Vec<Intent>,
) {
    let is_running = row.status.is_running();
    let words = if is_running { words::STOP } else { words::RUN };
    let mut button = Button::secondary(words).kind(ButtonKind::Icon);
    if !is_running && !can_run {
        button = button.disabled();
    }
    let (width, height) = ui.button_size(&button);
    let cell = place(line.right(width), (width, height), (Align::End, Align::Center));
    if ui.button(("run or stop the task", index), cell, &button).clicked {
        let asked = if is_running {
            TasksIntent::Stop(row.reference.clone())
        } else {
            TasksIntent::Run(row.reference.clone())
        };
        intents.push(tasks(asked));
    }
}

/// The context menu of a task: **Run**, **Stop**, **Restart**, **Open in tasks.json** and **Copy command**.
fn task_menu(
    ui: &mut Ui<'_>,
    response: &ui::Response,
    (index, row, can_run): (usize, &TaskRow, bool),
) -> Option<Intent> {
    let is_running = row.status.is_running();
    let able = |item: ui::MenuItem<'static, TaskCommand>, is_able: bool| {
        MenuEntry::Item(if is_able { item } else { item.disabled() })
    };
    let mut entries = vec![
        able(icons::without_icon(words::RUN, TaskCommand::Run), can_run),
        able(icons::action(icons::STOP, words::STOP, TaskCommand::Stop), is_running),
        able(
            icons::without_icon(words::RESTART, TaskCommand::Restart),
            is_running && can_run,
        ),
        MenuEntry::Separator,
    ];
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
    Some(tasks(
        match ui.context_menu(("task menu", index), response, &entries)? {
            TaskCommand::Run => TasksIntent::Run(reference),
            TaskCommand::Stop => TasksIntent::Stop(reference),
            TaskCommand::Restart => TasksIntent::Restart(reference),
            TaskCommand::OpenAt => TasksIntent::OpenAt(reference),
            TaskCommand::Copy => {
                ui.copy_to_clipboard(row.copied.clone());
                return None;
            }
        },
    ))
}
