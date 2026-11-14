//! What runs: the status bar at the foot of the window (`activity.status-bar`) and the Activity drawer
//! (`activity.what-is-running`), with the Command log in it (`log.show`).

mod drawer;
mod tasks;
#[cfg(test)]
mod tests;

use std::time::Duration;

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, Sense, TextKind, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::line::is_narrow;
use crate::text::header;

pub use drawer::{ActivityState, what_is_running_view};
pub use tasks::{TaskId, TaskIntent, TaskRow, TasksState};

/// How long something runs before the activity indicator shows it, so that what is over sooner does not flicker.
const SHOWN_AFTER: Duration = Duration::from_millis(500);
/// How often the times shown go on.
const TICK: Duration = Duration::from_millis(500);
/// The widest the words of the activity indicator are.
const WORDS_WIDTH: f32 = 240.0;
/// The narrowest the words of the activity indicator are; in less room it shows its icon alone.
const WORDS_LEAST_WIDTH: f32 = 80.0;

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

/// What the status bar shows.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StatusBarState<'a> {
    /// What runs, the oldest first.
    pub running: &'a [RunningTask],
    /// The identity commits are made with in the repository shown, once it is read.
    pub identity: Option<&'a model::state::Identity>,
    /// Whether an operation runs, while the identity cannot be switched.
    pub is_running: bool,
}

/// What has run long enough to be shown, the oldest first.
fn shown(running: &[RunningTask]) -> impl Iterator<Item = &RunningTask> {
    running.iter().filter(|task| task.elapsed >= SHOWN_AFTER)
}

/// The width the activity indicator takes out of the room it is given: none while nothing has run for half a second,
/// its icon alone when the window is narrow or its words would be cut to less than they say, and its words cut to the
/// room otherwise.
#[must_use]
fn indicator_width(state: &StatusBarState<'_>, ui: &Ui<'_>, room: f32) -> f32 {
    let Some(longest) = shown(state.running).next() else {
        return 0.0;
    };
    let theme = ui.theme();
    let icon = theme.sizes.icon;
    let words_room = room - icon - theme.sizes.spacing.medium;
    if is_narrow(ui) || words_room < WORDS_LEAST_WIDTH {
        return icon;
    }
    let words = header::running(&longest.title, longest.elapsed.as_secs());
    let words_width = ui
        .measure(&words, ui.text_style(TextKind::Detail))
        .min(WORDS_WIDTH)
        .min(words_room);
    icon + theme.sizes.spacing.medium + words_width
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

/// The status bar, over the rectangle its caller clipped it to: at its start the activity indicator, a spinning icon
/// with the title of what has run longest and for how long, once it has run for half a second, whose tooltip lists
/// everything that runs and whose click opens the Activity drawer; at its end the identity commits are made with, which
/// a click switches (`identity.switch`), when it fits after the indicator. It asks for the frames that make the
/// indicator appear and its times go on.
///
/// Feature: activity.status-bar
pub fn status_bar_view(state: &StatusBarState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    ask_for_next_frame(state.running, ui);
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let bar = ui.current_clip();
    let mut parts = Area::with_gap(inset(bar, Insets::symmetric(spacing.large, 0.0)), spacing.large);
    let room = parts.rest().width;
    let width = indicator_width(state, ui, room);
    let indicator = parts.left(width);
    let mut intents = Vec::new();
    if let Some(identity) = state.identity {
        let said = crate::text::changes::identity_said(identity.name.as_deref(), identity.email.as_deref());
        let mut link = Button::secondary(&said)
            .kind(ButtonKind::Icon)
            .with_tooltip(crate::text::changes::SWITCH_IDENTITY);
        if state.is_running {
            link = link.disabled();
        }
        let (width, height) = ui.button_size(&link);
        if width <= parts.rest().width {
            let cell = place(
                parts.right(width),
                (width, height.min(bar.height)),
                (Align::Start, Align::Center),
            );
            if ui.button("status identity", cell, &link).clicked {
                intents.push(Intent::Feature(FeatureIntent::Run(
                    FeatureId("identity.switch"),
                    Target::Head,
                )));
            }
        }
    }
    let Some(longest) = shown(state.running).next() else {
        return intents;
    };
    let words = header::running(&longest.title, longest.elapsed.as_secs());
    let mut pieces = Area::with_gap(indicator, spacing.medium);
    let mark = pieces.left(theme.sizes.icon);
    ui.spinner(place(
        mark,
        (mark.width, theme.sizes.icon),
        (Align::Start, Align::Center),
    ));
    let words_rect = pieces.rest();
    if words_rect.width > 0.0 {
        let _ = ui.label(words_rect, &Label::new(&words).kind(TextKind::Detail).tone(Tone::Muted));
    }
    let id = ui.id("activity indicator");
    let response = ui.interact(id, indicator, Sense::CLICK | Sense::FOCUS);
    ui.describe(id, indicator, &Description::new(AccessRole::Button, &words));
    ui.tooltip(&response, &tooltip(state.running));
    if response.clicked {
        intents.push(Intent::OpenActivity);
    }
    intents
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
