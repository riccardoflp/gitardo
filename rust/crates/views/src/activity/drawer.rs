//! The Activity drawer (`activity.what-is-running`): a card over the bottom of what is under the header, with what runs
//! and what ran a moment ago, the Command log beside them once it is asked for (`log.show`), and the Tasks once a new
//! worktree runs commands (`worktree-setup.command-steps`).

use ui::layout::{Align, Area, Insets, Size, columns, inset, place};
use ui::{AccessRole, Button, ButtonKind, Icon, Label, Layer, Rect, Sense, TextKind, Tone, Ui};

use super::tasks::{TasksState, tasks_column};
use super::{RunningTask, ask_for_next_frame};
use crate::intent::Intent;
use crate::log::{LogState, log_view};
use crate::text::activity as words;

/// The highest the drawer is.
const HEIGHT: f32 = 280.0;
/// The highest the drawer is while it shows the Tasks, whose output needs the room.
const HEIGHT_WITH_TASKS: f32 = 380.0;
/// The most of the height of what is under the header the drawer takes.
const MOST_OF_HEIGHT: f32 = 0.6;

/// What the Activity drawer lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityState<'a> {
    /// What runs, the oldest first.
    pub running: &'a [RunningTask],
    /// What ran a moment ago, the latest first, each with how long it took.
    pub finished: &'a [RunningTask],
    /// The Command log, once **Show the commands Gitardo runs** was clicked.
    pub log: Option<LogState<'a>>,
    /// The Tasks, once a new worktree ran commands.
    pub tasks: Option<TasksState<'a>>,
}

/// The rectangle of the drawer: along the bottom of the rectangle given, with a margin around it, at most as high as
/// `highest`.
fn drawer_rect(area: Rect, margin: f32, highest: f32) -> Rect {
    let height = highest.min(area.height * MOST_OF_HEIGHT);
    Rect::new(
        area.x + margin,
        area.bottom() - margin - height,
        (area.width - 2.0 * margin).max(0.0),
        height,
    )
}

/// The Activity drawer, over the bottom of the rectangle its caller clipped it to: its title and the button that
/// closes it; what runs, each with its title, how long it has run and its detail, or that nothing runs; what ran a
/// moment ago; **Show the commands Gitardo runs**, which shows the Command log beside them; then the Tasks, once a new
/// worktree ran commands. The times go on while it is open.
///
/// Feature: activity.what-is-running
pub fn what_is_running_view(state: &ActivityState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let margin = spacing.large + spacing.small;
    let highest = if state.tasks.is_some() {
        HEIGHT_WITH_TASKS
    } else {
        HEIGHT
    };
    let rect = drawer_rect(ui.current_clip(), margin, highest);
    ask_for_next_frame(state.running, ui);
    ui.layer(Layer::Popup, |ui| {
        ui.floating_card(rect, theme.sizes.radius.large + spacing.small);
        let _ = ui.interact(ui.id("activity drawer"), rect, Sense::CLICK);
        ui.region(
            "activity drawer region",
            (rect, AccessRole::Complementary, words::DRAWER),
            |ui| {
                let mut area = Area::with_gap(inset(rect, Insets::all(margin)), spacing.large);
                title_bar(ui, area.top(theme.sizes.control_height), &mut intents);
                match (&state.log, &state.tasks) {
                    (Some(log), Some(tasks)) => {
                        let sizes = [Size::Fill(1.0), Size::Fill(1.4), Size::Fill(1.4)];
                        let [running, log_column, tasks_rect] = columns(area.rest(), margin, sizes);
                        running_column(state, ui, running, &mut intents);
                        log_column_view(log, ui, log_column, &mut intents);
                        tasks_column(tasks, ui, tasks_rect, &mut intents);
                    }
                    (Some(log), None) => {
                        let [running, log_column] = columns(area.rest(), margin, [Size::Fill(1.0), Size::Fill(1.4)]);
                        running_column(state, ui, running, &mut intents);
                        log_column_view(log, ui, log_column, &mut intents);
                    }
                    (None, Some(tasks)) => {
                        let [running, tasks_rect] = columns(area.rest(), margin, [Size::Fill(1.0), Size::Fill(1.6)]);
                        running_column(state, ui, running, &mut intents);
                        tasks_column(tasks, ui, tasks_rect, &mut intents);
                    }
                    (None, None) => running_column(state, ui, area.rest(), &mut intents),
                }
            },
        );
    });
    intents
}

/// The title of the drawer and the button that closes it.
fn title_bar(ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let mut area = Area::new(rect);
    let close = Button::icon(Icon::Close, words::CLOSE_DRAWER).kind(ButtonKind::Icon);
    if ui.button("close activity", area.right(rect.height), &close).clicked {
        intents.push(Intent::CloseActivity);
    }
    let _ = ui.label(area.rest(), &Label::new(words::DRAWER).kind(TextKind::Strong));
}

/// The title of a column of the drawer.
fn column_title(ui: &mut Ui<'_>, area: &mut Area, text: &str) {
    let height = ui.theme().sizes.row_height;
    let label = Label::new(text).kind(TextKind::SmallStrong).tone(Tone::Muted);
    let _ = ui.label(area.top(height), &label);
}

/// What runs and what ran, then the button that shows the Command log when it is not shown.
fn running_column(state: &ActivityState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    column_title(ui, &mut area, words::RUNNING_COLUMN);
    if state.log.is_none() {
        let show = Button::secondary(words::SHOW_COMMANDS).kind(ButtonKind::Outline);
        let (width, height) = ui.button_size(&show);
        let cell = place(area.bottom(height), (width, height), (Align::Start, Align::Center));
        if ui.button("show the commands", cell, &show).clicked {
            intents.push(Intent::ShowCommandLog);
        }
    }
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
        ui.icon_in(mark, Icon::Check, Tone::Success);
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

/// The Command log, under its title, with the button that puts it away.
fn log_column_view(log: &LogState<'_>, ui: &mut Ui<'_>, rect: Rect, intents: &mut Vec<Intent>) {
    let theme = *ui.theme();
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    let mut top = Area::new(area.top(theme.sizes.row_height));
    let hide = Button::secondary(words::HIDE_COMMANDS).kind(ButtonKind::Icon);
    let (width, height) = ui.button_size(&hide);
    let cell = place(top.right(width), (width, height), (Align::End, Align::Center));
    if ui.button("hide the commands", cell, &hide).clicked {
        intents.push(Intent::HideCommandLog);
    }
    let label = Label::new(words::LOG_COLUMN)
        .kind(TextKind::SmallStrong)
        .tone(Tone::Muted);
    let _ = ui.label(top.rest(), &label);
    let body = area.rest();
    intents.extend(ui.clip(body, |ui| log_view(log, ui)));
}
