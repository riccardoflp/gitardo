//! The Activity tab of the bottom panel (`activity.what-is-running`, D64): what runs and what ran a moment ago, and at
//! its right the Tasks once a new worktree runs commands (`worktree-setup.command-steps`). The Command log is the tab
//! beside it (`log.show`, `panel.bottom`).

use ui::layout::{Align, Area, Insets, Size, columns, inset, place};
use ui::{AccessRole, Label, Rect, TextKind, Tone, Ui};

use super::tasks::{TasksState, tasks_column};
use super::{RunningTask, ask_for_next_frame};
use crate::icons;
use crate::intent::Intent;
use crate::text::activity as words;

/// What the Activity tab lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityState<'a> {
    /// What runs, the oldest first.
    pub running: &'a [RunningTask],
    /// What ran a moment ago, the latest first, each with how long it took.
    pub finished: &'a [RunningTask],
    /// The Tasks, once a new worktree ran commands.
    pub tasks: Option<TasksState<'a>>,
}

/// The Activity tab, over the rectangle its caller clipped it to: what runs, each with its title, how long it has run
/// and its detail, or that nothing runs; what ran a moment ago; and at its right the Tasks, once a new worktree ran
/// commands. The times go on while it is shown.
///
/// Feature: activity.what-is-running
pub fn what_is_running_view(state: &ActivityState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let spacing = ui.theme().sizes.spacing;
    let rect = ui.current_clip();
    ask_for_next_frame(state.running, ui);
    ui.region("activity tab", (rect, AccessRole::Region, words::ACTIVITY), |ui| {
        let inside = inset(rect, Insets::symmetric(spacing.medium, 0.0));
        if let Some(tasks) = &state.tasks {
            let gap = spacing.large + spacing.small;
            let [running, tasks_rect] = columns(inside, gap, [Size::Fill(1.0), Size::Fill(1.6)]);
            running_column(state, ui, running);
            tasks_column(tasks, ui, tasks_rect, &mut intents);
        } else {
            running_column(state, ui, inside);
        }
    });
    intents
}

/// The title of a column of the tab.
fn column_title(ui: &mut Ui<'_>, area: &mut Area, text: &str) {
    let height = ui.theme().sizes.row_height;
    let label = Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted);
    let _ = ui.label(area.top(height), &label);
}

/// What runs and what ran.
fn running_column(state: &ActivityState<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    column_title(ui, &mut area, words::RUNNING_COLUMN);
    let mut rows = Area::new(area.rest());
    if state.running.is_empty() {
        let _ = ui.label(rows.top(theme.sizes.row_height), &Label::new(words::NOTHING_RUNNING));
        let detail = Label::new(words::NOTHING_RUNNING_DETAIL)
            .kind(TextKind::Detail)
            .tone(Tone::Muted);
        let _ = ui.label(rows.top(theme.sizes.row_height), &detail);
    }
    for task in state.running {
        task_row(ui, &mut rows, task, false);
    }
    for task in state.finished {
        task_row(ui, &mut rows, task, true);
    }
}

/// A line of what runs, or ran: a spinner or a check, the title and the time; under it the detail, when there is one.
fn task_row(ui: &mut Ui<'_>, rows: &mut Area, task: &RunningTask, is_finished: bool) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut line = Area::with_gap(rows.top(theme.sizes.row_height), spacing.medium);
    let mark = place(
        line.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    let seconds = task.elapsed.as_secs();
    let time = if is_finished {
        ui.icon_in(mark, icons::CHOSEN, Tone::Success);
        words::took(seconds)
    } else {
        ui.spinner(mark);
        words::duration(seconds)
    };
    let detail_style = ui.text_style(TextKind::Detail);
    let time_width = ui.measure(&time, detail_style);
    let _ = ui.label(
        line.right(time_width),
        &Label::new(&time).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let title = Label::new(&task.title).tone(if is_finished { Tone::Muted } else { Tone::Normal });
    let _ = ui.label(line.rest(), &title);
    if let Some(detail) = &task.detail {
        let under = rows.top(theme.sizes.row_height);
        let under = Rect::new(
            under.x + theme.sizes.icon + spacing.medium,
            under.y,
            (under.width - theme.sizes.icon - spacing.medium).max(0.0),
            under.height,
        );
        let _ = ui.label(under, &Label::new(detail).kind(TextKind::Detail).tone(Tone::Muted));
    }
}
