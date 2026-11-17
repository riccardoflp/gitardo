//! The Tasks of the Activity tab (`worktree-setup.command-steps`): the commands of new worktrees, one after the
//! other, each named after its worktree, and beside them what the one shown printed as it comes, with the box that
//! answers what the one running asks, and **Stop**.

use model::tasks::{TaskEnd, TaskStatus};
use ui::layout::{Align, Area, Insets, Size, columns, inset, place};
use ui::{
    AccessRole, Button, ButtonKind, Description, InputBox, Key, Label, List, Modifiers, Point, Rect, Reveal,
    RevealPlace, SelectionMode, Sense, Steering, TextKind, TextRun, Tone, Ui,
};

use crate::icons;
use crate::intent::Intent;
use crate::text::activity as words;

/// The key of the list of the lines of the output.
const OUTPUT: &str = "task output";

/// A task, by the number of the commands of a worktree it is one of, and its place among them.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaskId {
    /// The commands of one worktree, numbered in the order they started.
    pub batch: u64,
    /// Its place among them, from 0.
    pub index: usize,
}

/// A row of the Tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRow {
    /// Which task.
    pub id: TaskId,
    /// Its command: `npm ci`.
    pub command: String,
    /// The worktree it runs in, as the views name it.
    pub name: String,
    /// How it stands.
    pub status: TaskStatus,
}

/// What the Tasks show.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TasksState<'a> {
    /// The tasks, in the order they run.
    pub rows: &'a [TaskRow],
    /// The task whose output is shown.
    pub shown: Option<TaskId>,
    /// What it printed, in lines, oldest first.
    pub output: &'a [String],
    /// How many older lines of it were dropped.
    pub dropped: usize,
    /// What is typed in the box that answers the task running.
    pub typed: &'a str,
}

/// What is asked of the Tasks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TaskIntent {
    /// Show what a task printed.
    Show(TaskId),
    /// The text of the box that answers the task running changed.
    Typed(String),
    /// `Enter` in that box: give what is typed to the task running.
    Answer,
    /// **Stop**: stop the task running, so that those after it do not run.
    Stop(TaskId),
}

/// The task running among the rows, if one is.
fn running<'a>(state: &TasksState<'a>) -> Option<&'a TaskRow> {
    state.rows.iter().find(|row| row.status == TaskStatus::Running)
}

/// The Tasks, in the rectangle given: their title with **Stop** while one runs; under it, on the left a row for each
/// of the latest, as many as fit, and on the right what the task shown printed, following its end, over the box that
/// answers the task running while it is the one shown.
pub(super) fn tasks_column(state: &TasksState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut area = Area::with_gap(rect, spacing.medium);
    let mut top = Area::new(area.top(theme.sizes.row_height));
    let running = running(state);
    if let Some(row) = running {
        let stop = Button::secondary(words::STOP_TASK)
            .kind(ButtonKind::Icon)
            .with_tooltip(words::STOP_TASK_TOOLTIP);
        let (width, height) = ui.button_size(&stop);
        let cell = place(top.right(width), (width, height), (Align::End, Align::Center));
        if ui.button("stop the task", cell, &stop).clicked {
            intents.push(Intent::Tasks(TaskIntent::Stop(row.id)));
        }
    }
    let title = Label::new(words::TASKS_COLUMN)
        .kind(TextKind::SmallStrong)
        .tone(Tone::Muted);
    let _ = ui.label(top.rest(), &title);
    let [rows, printed] = columns(area.rest(), spacing.medium, [Size::Fill(1.0), Size::Fill(1.4)]);
    let mut rows = Area::with_gap(rows, spacing.small);
    for row in latest_rows(state, rows_that_fit(rows.rest().height, &theme)) {
        task_row(
            ui,
            rows.top(theme.sizes.row_height),
            (row, state.shown == Some(row.id)),
            intents,
        );
    }
    let mut printed = Area::with_gap(printed, spacing.medium);
    let answering = running.filter(|row| state.shown == Some(row.id));
    if let Some(row) = answering {
        answer_box(state, ui, printed.bottom(theme.sizes.control_height), (row, intents));
    }
    output_box(state, ui, printed.rest());
}

/// How many rows of tasks fit in a height, one at least.
fn rows_that_fit(height: f32, theme: &ui::Theme) -> usize {
    let row = theme.sizes.row_height + theme.sizes.spacing.small;
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a height in rows is a small number, never negative"
    )]
    let count = ((height + theme.sizes.spacing.small) / row).floor().max(1.0) as usize;
    count
}

/// The latest rows of the tasks, as many as a count.
fn latest_rows<'a>(state: &TasksState<'a>, count: usize) -> &'a [TaskRow] {
    state
        .rows
        .get(state.rows.len().saturating_sub(count)..)
        .unwrap_or_default()
}

/// A row of a task: the mark of how it stands, its command, its worktree and, when it did not succeed, why. A click
/// shows what it printed.
fn task_row(ui: &mut Ui<'_>, rect: Rect, (row, is_shown): (&TaskRow, bool), intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let id = ui.id(("task row", row.id));
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    if is_shown {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_inactive_selection);
    } else if response.hovered {
        ui.fill_rounded_rect(rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let mut line = Area::with_gap(inset(rect, Insets::symmetric(spacing.small, 0.0)), spacing.medium);
    let mark = place(
        line.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    let said = match &row.status {
        TaskStatus::Waiting => (Some((icons::WAITS, Tone::Muted)), words::TASK_WAITS.to_owned()),
        TaskStatus::Running => (None, String::new()),
        TaskStatus::Succeeded => (Some((icons::CHOSEN, Tone::Success)), String::new()),
        TaskStatus::Failed(TaskEnd::Code(code)) => (Some((icons::ERROR, Tone::Error)), words::exit_code(*code)),
        TaskStatus::Failed(TaskEnd::Stopped) => (Some((icons::WARNING, Tone::Warning)), words::TASK_STOPPED.to_owned()),
        TaskStatus::Failed(TaskEnd::NotStarted(_)) => {
            (Some((icons::ERROR, Tone::Error)), words::TASK_NOT_STARTED.to_owned())
        }
        TaskStatus::DidNotRun => (Some((icons::SKIPPED, Tone::Muted)), words::TASK_DID_NOT_RUN.to_owned()),
    };
    match said.0 {
        Some((icon, tone)) => ui.icon_in(mark, icon, tone),
        None => ui.spinner(mark),
    }
    let detail = if said.1.is_empty() {
        row.name.clone()
    } else {
        format!("{} \u{b7} {}", row.name, said.1)
    };
    let detail_width = ui
        .measure(&detail, ui.text_style(TextKind::Detail))
        .min(line.rest().width * 0.5);
    let _ = ui.label(
        line.right(detail_width),
        &Label::new(&detail).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let _ = ui.label(line.rest(), &Label::new(&row.command).kind(TextKind::Code));
    let name = format!("{}, {detail}", row.command);
    ui.describe(id, rect, &Description::new(AccessRole::Button, &name));
    if response.clicked {
        intents.push(Intent::Tasks(TaskIntent::Show(row.id)));
    }
}

/// The box that gives the task running what is typed, with `Enter`.
fn answer_box(state: &TasksState<'_>, ui: &mut Ui<'_>, rect: Rect, (row, intents): (&TaskRow, &mut Vec<Intent>)) {
    let placeholder = words::answer_task(&row.command);
    let typed = ui.input_box(
        ("answer the task", row.id),
        rect,
        &InputBox::new(state.typed, &placeholder),
    );
    if let Some(text) = typed.changed_text {
        intents.push(Intent::Tasks(TaskIntent::Typed(text)));
    }
    if typed.is_submitted {
        intents.push(Intent::Tasks(TaskIntent::Answer));
    }
}

/// What the task shown printed, in a box whose list follows the last line as lines come; `Ctrl+C` copies the lines
/// selected.
fn output_box(state: &TasksState<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rounded_rect(rect, theme.sizes.radius.large, theme.colors.secondary_button_background);
    let inside = inset(rect, Insets::all(spacing.medium));
    if state.output.is_empty() {
        let label = Label::new(words::NOTHING_PRINTED)
            .kind(TextKind::Detail)
            .tone(Tone::Muted);
        let _ = ui.label(
            Rect::new(inside.x, inside.y, inside.width, theme.sizes.row_height),
            &label,
        );
        return;
    }
    let style = ui.text_style(TextKind::Code);
    let list = List {
        name: words::TASKS,
        row_count: state.output.len(),
        row_height: ui.line_height(style) + spacing.tiny,
        selection_mode: SelectionMode::Multiple,
        lead: 0.0,
    };
    let copy = [(Key::Character('c'), Modifiers::CONTROL)];
    let total = u64::try_from(state.output.len() + state.dropped).unwrap_or(u64::MAX);
    let steering = Steering {
        selection: None,
        reveal: Some(Reveal {
            serial: total,
            row: state.output.len().saturating_sub(1),
            place: RevealPlace::Nearest,
        }),
        left_to_view: &copy,
    };
    let _ = ui.steered_list((OUTPUT, state.shown), (inside, &list), &steering, |ui, row| {
        let Some(line) = state.output.get(row.index) else {
            return;
        };
        let _ = ui.text(&TextRun {
            text: line,
            style,
            color: row.foreground,
            origin: Point::new(row.rect.x + spacing.small, row.rect.y + spacing.tiny * 0.5),
            max_width: Some(row.rect.width - spacing.small * 2.0),
        });
    });
    copy_selected(state, ui);
}

/// Puts the lines selected on the clipboard when `Ctrl+C` was pressed in the output.
fn copy_selected(state: &TasksState<'_>, ui: &mut Ui<'_>) {
    let key = (OUTPUT, state.shown);
    let id = ui.id(key);
    let is_copied = ui
        .unclaimed_keys_of(id)
        .iter()
        .any(|key| key.modifiers == Modifiers::CONTROL && key.key == Key::Character('c'));
    if !is_copied {
        return;
    }
    let selection = ui.list_selection(key);
    let copied: Vec<&str> = state
        .output
        .iter()
        .enumerate()
        .filter(|(index, _)| selection.contains(*index))
        .map(|(_, line)| line.as_str())
        .collect();
    if !copied.is_empty() {
        ui.copy_to_clipboard(copied.join("\n"));
    }
}
