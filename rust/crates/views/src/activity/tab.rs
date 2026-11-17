//! The Activity tab of the bottom panel (`activity.what-is-running`, D64): what runs, each row where it is while it
//! runs, and under **Ran** what ran, the latest first, in a list that scrolls; and at its right the Tasks once a new
//! worktree runs commands (`worktree-setup.command-steps`). The Command log is the tab beside it (`log.show`,
//! `panel.bottom`).

use ui::layout::{Align, Area, Insets, Size, columns, inset, place};
use ui::{AccessRole, Label, Rect, TextKind, Tone, Ui};

use super::tasks::{TasksState, tasks_column};
use super::{RanTask, RunningTask, ask_for_next_frame, shown};
use crate::icons;
use crate::intent::Intent;
use crate::text::activity as words;

/// What the Activity tab lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityState<'a> {
    /// What runs, in the order it started; what has not run for half a second yet is not listed.
    pub running: &'a [RunningTask],
    /// What ran, the latest first, each with when it ended and how long it took.
    pub ran: &'a [RanTask],
    /// The Tasks, once a new worktree ran commands.
    pub tasks: Option<TasksState<'a>>,
}

/// A line of the list of the tab.
#[derive(Clone, Copy)]
enum Line<'a> {
    /// That nothing runs, said where the first thing running would be.
    NothingRuns,
    /// Something running.
    Running(&'a RunningTask),
    /// The heading of what ran.
    RanHeading,
    /// Something that ran.
    Ran(&'a RanTask),
}

impl Line<'_> {
    /// How high the line is, in rows: a heading is one, anything else a title over its detail, whether it has one or
    /// not, so that a row is as high whatever it says.
    fn rows(self) -> f32 {
        match self {
            Line::RanHeading => 1.0,
            Line::NothingRuns | Line::Running(_) | Line::Ran(_) => 2.0,
        }
    }
}

/// The Activity tab, over the rectangle its caller clipped it to: what has run for half a second, in the order it
/// started, each with its title, how long it has run and its detail, or that nothing runs; under **Ran** what ran, the
/// latest first, each with the time it ended and how long it took; the list scrolls when it is taller than the tab.
/// At its right the Tasks, once a new worktree ran commands. The times go on while it is shown.
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

/// The lines of the list, from the top: what runs, or that nothing does, then what ran under its heading.
fn lines<'a>(state: &ActivityState<'a>) -> Vec<Line<'a>> {
    let mut lines: Vec<Line<'a>> = shown(state.running).map(Line::Running).collect();
    if lines.is_empty() {
        lines.push(Line::NothingRuns);
    }
    if !state.ran.is_empty() {
        lines.push(Line::RanHeading);
        lines.extend(state.ran.iter().map(Line::Ran));
    }
    lines
}

/// What runs and what ran, under the title of the column, in a list that scrolls when it is taller than the tab.
fn running_column(state: &ActivityState<'_>, ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    column_title(ui, &mut area, words::RUNNING_COLUMN);
    let list = area.rest();
    let lines = lines(state);
    let row = theme.sizes.row_height;
    let height: f32 = lines.iter().map(|line| line.rows() * row).sum();
    let _ = ui.scroll_area("activity list", list, (list.width, height), |ui, content| {
        // The scroll bar is drawn over the end of the rows: the times stop before it.
        let width = (content.width - theme.sizes.scrollbar).max(0.0);
        let mut y = content.y;
        for line in &lines {
            let line_rect = Rect::new(content.x, y, width, line.rows() * row);
            y += line_rect.height;
            if line_rect.bottom() > list.y && line_rect.y < list.bottom() {
                draw_line(ui, line_rect, *line);
            }
        }
    });
}

/// Draws a line of the list in its rectangle.
fn draw_line(ui: &mut Ui<'_>, rect: Rect, line: Line<'_>) {
    let mut rows = Area::new(rect);
    let row = ui.theme().sizes.row_height;
    match line {
        Line::NothingRuns => {
            let _ = ui.label(rows.top(row), &Label::new(words::NOTHING_RUNNING));
            let detail = Label::new(words::NOTHING_RUNNING_DETAIL)
                .kind(TextKind::Detail)
                .tone(Tone::Muted);
            let _ = ui.label(rows.top(row), &detail);
        }
        Line::Running(task) => {
            let time = words::duration(task.elapsed.as_secs());
            task_row(ui, &mut rows, (task, &time), false);
        }
        Line::RanHeading => {
            let label = Label::new(words::RAN_HEADING)
                .kind(TextKind::SmallStrong)
                .tone(Tone::Muted);
            let _ = ui.label(rows.top(row), &label);
        }
        Line::Ran(ran) => {
            let time = words::ran(ran.ended, ran.task.elapsed);
            task_row(ui, &mut rows, (&ran.task, &time), true);
        }
    }
}

/// A row of what runs, or ran: a spinner or a check, the title and the time; under it the detail, when there is one.
fn task_row(ui: &mut Ui<'_>, rows: &mut Area, (task, time): (&RunningTask, &str), is_ran: bool) {
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut line = Area::with_gap(rows.top(theme.sizes.row_height), spacing.medium);
    let mark = place(
        line.left(theme.sizes.icon),
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Start, Align::Center),
    );
    if is_ran {
        ui.icon_in(mark, icons::CHOSEN, Tone::Success);
    } else {
        ui.spinner(mark);
    }
    let detail_style = ui.text_style(TextKind::Detail);
    let time_width = ui.measure(time, detail_style);
    let _ = ui.label(
        line.right(time_width),
        &Label::new(time).kind(TextKind::Detail).tone(Tone::Muted),
    );
    let title = Label::new(&task.title).tone(if is_ran { Tone::Muted } else { Tone::Normal });
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
