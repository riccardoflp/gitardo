//! What runs: the activity indicator of the header (`activity.status-bar`) and the Activity drawer
//! (`activity.what-is-running`), with the Command log in it (`log.show`).

mod drawer;
#[cfg(test)]
mod tests;

use std::time::Duration;

use ui::layout::{Align, Area, place};
use ui::{AccessRole, Description, Label, Sense, TextKind, Tone, Ui};

use crate::intent::Intent;
use crate::line::is_narrow;
use crate::text::header;

pub use drawer::{ActivityState, what_is_running_view};

/// How long something runs before the activity indicator shows it, so that what is over sooner does not flicker.
const SHOWN_AFTER: Duration = Duration::from_millis(500);
/// How often the times shown go on.
const TICK: Duration = Duration::from_millis(500);
/// The widest the words of the activity indicator are.
const WORDS_WIDTH: f32 = 240.0;

/// Something running, or that ran, as the header and the drawer say it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunningTask {
    /// What it is: `Reading app`.
    pub title: String,
    /// Why it runs: `Because src/a.rs changed.`; none when nothing is said.
    pub detail: Option<String>,
    /// How long it has run, when the frame is drawn; for what ran, how long it took.
    pub elapsed: Duration,
}

/// What the activity indicator of the header shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusBarState<'a> {
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

/// The tooltip of the activity indicator: everything that runs, the oldest first, each with its title, how long it has
/// run and its detail, then **Click to open the Activity drawer.**
#[must_use]
pub fn tooltip(running: &[RunningTask]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for task in running {
        lines.push(header::running(&task.title, task.elapsed.as_secs()));
        if let Some(detail) = &task.detail {
            lines.push(detail.clone());
        }
    }
    lines.push(header::OPEN_THE_DRAWER.to_owned());
    lines.join("\n")
}

/// The activity indicator, over the rectangle its caller clipped it to: a spinning icon with the title of what has run
/// longest and for how long, once it has run for half a second; its tooltip lists everything that runs, and a click
/// opens the Activity drawer. It asks for the frames that make it appear and its times go on.
///
/// Feature: activity.status-bar
pub fn status_bar_view(state: &StatusBarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    ask_for_next_frame(state.running, ui);
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
    ui.tooltip(&response, &tooltip(state.running));
    if response.clicked {
        vec![Intent::OpenActivity]
    } else {
        Vec::new()
    }
}

/// Asks for the frame at which the indicator appears, or at which its times go on.
fn ask_for_next_frame(running: &[RunningTask], ui: &mut Ui<'_>) {
    let wait = running
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
