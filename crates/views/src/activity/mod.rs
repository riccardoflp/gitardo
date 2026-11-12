//! What runs: the activity indicator of the header (`activity.status-bar`) and the Activity drawer
//! (`activity.what-is-running`).

use std::time::Duration;

use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Description, Icon, Label, Layer, Rect, Sense, TextKind, Tone, Ui};

use crate::intent::Intent;
use crate::line::is_narrow;
use crate::text::activity as words;
use crate::text::header;

/// How long something runs before the activity indicator shows it, so that what is over sooner does not flicker.
const SHOWN_AFTER: Duration = Duration::from_millis(500);
/// How often the times shown go on.
const TICK: Duration = Duration::from_millis(500);
/// The widest the words of the activity indicator are.
const WORDS_WIDTH: f32 = 240.0;
/// The width of the Activity drawer.
const DRAWER_WIDTH: f32 = 360.0;

/// Something running, as the header and the drawer say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTask {
    /// What it is: `Reading app`.
    pub title: String,
    /// How long it has run, when the frame is drawn.
    pub elapsed: Duration,
}

/// What the activity indicator of the header shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusBarState<'a> {
    /// What runs, the oldest first.
    pub running: &'a [RunningTask],
}

/// What the Activity drawer lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ActivityState<'a> {
    /// What runs, the oldest first.
    pub running: &'a [RunningTask],
}

/// What has run long enough to be shown, the oldest first.
fn shown(running: &[RunningTask]) -> impl Iterator<Item = &RunningTask> {
    running.iter().filter(|task| task.elapsed >= SHOWN_AFTER)
}

/// The width the activity indicator takes in the header: none while nothing has run for half a second.
#[must_use]
pub fn status_bar_width(state: &StatusBarState<'_>, ui: &Ui<'_>) -> f32 {
    let Some(longest) = shown(state.running).next() else {
        return 0.0;
    };
    let theme = ui.theme();
    if is_narrow(ui) {
        return theme.sizes.icon;
    }
    let words = header::running(&longest.title, longest.elapsed.as_secs());
    let words_width = ui.measure(&words, ui.text_style(TextKind::Detail)).min(WORDS_WIDTH);
    theme.sizes.icon + theme.sizes.spacing.medium + words_width
}

/// The activity indicator, over the rectangle its caller clipped it to: a spinning icon with the title of what has run
/// longest and for how long, once it has run for half a second; its tooltip lists everything that runs, and a click
/// opens the Activity drawer. It asks for the frames that make it appear and its times go on.
///
/// Feature card that fills it further: `activity.status-bar` (the details of each task, from
/// `activity.describe-task`).
pub fn status_bar_view(state: &StatusBarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    ask_for_next_frame(state, ui);
    let rect = ui.current_clip();
    let Some(longest) = shown(state.running).next() else {
        return Vec::new();
    };
    let theme = *ui.theme();
    let words = header::running(&longest.title, longest.elapsed.as_secs());
    let mut parts = Area::with_gap(rect, theme.sizes.spacing.medium);
    let mark = parts.left(theme.sizes.icon);
    ui.spinner(place(
        mark,
        (mark.width, theme.sizes.icon),
        (Align::Start, Align::Center),
    ));
    if !is_narrow(ui) {
        let _ = ui.label(
            parts.rest(),
            &Label::new(&words).kind(TextKind::Detail).tone(Tone::Muted),
        );
    }
    let id = ui.id("activity indicator");
    let response = ui.interact(id, rect, Sense::CLICK | Sense::FOCUS);
    ui.describe(id, rect, &Description::new(AccessRole::Button, &words));
    let mut tooltip: Vec<String> = shown(state.running)
        .map(|task| header::running(&task.title, task.elapsed.as_secs()))
        .collect();
    tooltip.push(header::OPEN_THE_DRAWER.to_owned());
    ui.tooltip(&response, &tooltip.join("\n"));
    if response.clicked {
        vec![Intent::OpenActivity]
    } else {
        Vec::new()
    }
}

/// Asks for the frame at which the indicator appears, or at which its times go on.
fn ask_for_next_frame(state: &StatusBarState<'_>, ui: &mut Ui<'_>) {
    let wait = state
        .running
        .iter()
        .map(|task| {
            if task.elapsed < SHOWN_AFTER {
                SHOWN_AFTER.saturating_sub(task.elapsed)
            } else {
                let into_tick = task.elapsed.as_millis() % TICK.as_millis();
                TICK.saturating_sub(Duration::from_millis(u64::try_from(into_tick).unwrap_or_default()))
            }
        })
        .min();
    if let Some(wait) = wait {
        let at = ui.now() + wait;
        ui.request_frame_at(at);
    }
}

/// The Activity drawer, along the right of the rectangle its caller clipped it to: what runs, each with its title and
/// how long it has run, or that nothing runs.
///
/// Feature card that fills it further: `activity.what-is-running` (the detail of each task, **Show the commands
/// Gitardo runs** and the Command log of `log.show`, the Tasks of new worktrees).
pub fn what_is_running_view(state: &ActivityState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let area = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let width = DRAWER_WIDTH.min(area.width);
    let drawer = Rect::new(area.right() - width, area.y, width, area.height);
    ask_for_next_frame(&StatusBarState { running: state.running }, ui);
    ui.layer(Layer::Popup, |ui| {
        ui.fill_rect(drawer, theme.colors.drawer_background);
        let _ = ui.interact(ui.id("activity drawer"), drawer, Sense::CLICK);
        ui.region(
            "activity drawer region",
            (drawer, AccessRole::Complementary, words::DRAWER),
            |ui| {
                let mut column =
                    Area::with_gap(inset(drawer, Insets::all(spacing.large + spacing.small)), spacing.large);
                let top = column.top(theme.sizes.control_height);
                let mut bar = Area::with_gap(top, spacing.medium);
                let close = bar.right(theme.sizes.control_height);
                let button = Button::icon(Icon::Close, words::CLOSE_DRAWER).kind(ButtonKind::Icon);
                if ui.button("close activity", close, &button).clicked {
                    intents.push(Intent::CloseActivity);
                }
                let _ = ui.heading(bar.rest(), words::DRAWER);
                running_list(state.running, ui, column.rest());
            },
        );
    });
    intents
}

/// The list of what runs in the drawer, or what it says with nothing running.
fn running_list(running: &[RunningTask], ui: &mut Ui<'_>, rect: Rect) {
    let theme = *ui.theme();
    let mut rows = Area::new(rect);
    if running.is_empty() {
        let _ = ui.label(rows.top(theme.sizes.row_height), &Label::new(words::NOTHING_RUNNING));
        let detail = Label::new(words::NOTHING_RUNNING_DETAIL)
            .kind(TextKind::Detail)
            .tone(Tone::Muted);
        let _ = ui.label(rows.top(theme.sizes.row_height), &detail);
        return;
    }
    for task in running {
        let mut row = Area::with_gap(rows.top(theme.sizes.row_height), theme.sizes.spacing.medium);
        let time = words::duration(task.elapsed.as_secs());
        let time_width = ui.measure(&time, ui.text_style(TextKind::Detail));
        let _ = ui.label(
            row.right(time_width),
            &Label::new(&time).kind(TextKind::Detail).tone(Tone::Muted),
        );
        let _ = ui.label(row.rest(), &Label::new(&task.title));
    }
}
