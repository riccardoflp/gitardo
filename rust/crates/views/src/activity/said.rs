//! What was said, in the status bar (`activity.said`, D56): what an operation said when it ended and the notices of the
//! window, after what is running, as a line of the status bar's text with its links.

use std::time::Duration;

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{AccessRole, Button, ButtonKind, Description, Label, Rect, Sense, TextKind, Tone, Ui};

use crate::feature::{FeatureIntent, SaidState};
use crate::icons;
use crate::intent::Intent;
use crate::text::feature as words;

/// How long what succeeded is said once the pointer is not on it.
pub const SAID_FOR: Duration = Duration::from_secs(10);
/// How often the moment the pointer was last on it is kept while it is.
const KEPT_EVERY: Duration = Duration::from_secs(1);

/// The links of what was said, in their order from the left, each with what a click asks.
fn links(state: &SaidState<'_>) -> Vec<(String, Option<Intent>)> {
    let mut links: Vec<(String, Option<Intent>)> = Vec::new();
    if let Some(stopped) = &state.stopped {
        links.extend(
            stopped
                .buttons()
                .into_iter()
                .map(|(label, intent)| (label.to_owned(), intent)),
        );
        return links;
    }
    if state.can_undo {
        let undo = FeatureIntent::Run(FeatureId("undo.last"), Target::Head);
        links.push((words::UNDO.to_owned(), Some(Intent::Feature(undo))));
    }
    if let Some((feature, label)) = state.offer {
        links.push((
            label.to_owned(),
            Some(Intent::Feature(FeatureIntent::Run(feature, Target::Head))),
        ));
    }
    if let Some((label, url)) = state.link {
        links.push((label.to_owned(), Some(Intent::OpenUrl(url.to_owned()))));
    }
    for run in state.runs {
        let intent = FeatureIntent::RunWith(run.feature, run.target.clone(), run.answers.clone());
        links.push((run.label.as_str().to_owned(), Some(Intent::Feature(intent))));
    }
    if let Some(found) = state.found {
        links.push((
            crate::text::bisect::SHOW_IN_TIMELINE.to_owned(),
            Some(Intent::Graph(crate::graph::GraphIntent::GoTo(found.clone()))),
        ));
        links.push((
            crate::text::bisect::END_BISECT.to_owned(),
            Some(Intent::Feature(FeatureIntent::Run(
                FeatureId("bisect.end"),
                Target::Head,
            ))),
        ));
    }
    if state.offers_stashes {
        links.push((
            crate::text::stash::SHOW_STASHES.to_owned(),
            Some(Intent::Rail(crate::intent::RailIntent::ShowStashes)),
        ));
    }
    links
}

/// The words said, and the icon and the tone of what is said: an error, a warning, or what was done.
fn words_and_tone(state: &SaidState<'_>) -> (String, ui::Icon, Tone) {
    if let Some(stopped) = &state.stopped {
        return (stopped.says(), icons::WARNING, Tone::Warning);
    }
    let (icon, tone) = if state.is_error {
        (icons::ERROR, Tone::Error)
    } else if state.lasts() {
        (icons::WARNING, Tone::Warning)
    } else {
        (icons::INFORMATION, Tone::Success)
    };
    (state.text.to_owned(), icon, tone)
}

/// What was said, over the rectangle its caller clipped it to in the status bar: the icon of its tone, its words cut
/// with `…` where the room ends and whole in their tooltip, which a click opens the Command log at, at the command that
/// said it; then its links,
/// **Undo** and what it offers next, and **×**, which puts it away. What succeeded goes ten seconds after it was said,
/// or ten seconds after the pointer left it; a warning or an error stays.
///
/// Feature: activity.said
pub fn said_view(state: &SaidState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut intents = Vec::new();
    let mut parts = Area::with_gap(rect, spacing.medium);
    let (said, icon, tone) = words_and_tone(state);
    ui.icon_in(parts.left(theme.sizes.icon), icon, tone);
    let dismiss = if state.is_notice {
        Intent::DismissNotice
    } else {
        Intent::Feature(FeatureIntent::Dismiss)
    };
    let close = Button::icon(icons::CLOSE, words::DISMISS);
    if link(ui, &mut parts, ("said dismiss", &close)) {
        intents.push(dismiss);
    }
    for (index, (label, intent)) in links(state).into_iter().enumerate().rev() {
        let mut button = Button::secondary(&label).kind(ButtonKind::Icon);
        if intent.is_none() {
            button = button.disabled();
        }
        if link(ui, &mut parts, (&format!("said link {index}"), &button)) {
            intents.extend(intent);
        }
    }
    let text = parts.rest();
    let text_tone = if tone == Tone::Error { Tone::Error } else { Tone::Normal };
    let _ = ui.label(text, &Label::new(&said).kind(TextKind::Detail).tone(text_tone));
    let id = ui.id("said words");
    let response = ui.interact(id, text, Sense::CLICK);
    ui.describe(id, text, &Description::new(AccessRole::Button, &said));
    ui.tooltip(&response, &said);
    if response.clicked {
        intents.push(state.log_at.map_or(Intent::ShowCommandLog, |line| {
            Intent::Panel(crate::panel::PanelIntent::ShowLogAt(line))
        }));
    }
    intents.extend(said_for(state, ui, rect));
    intents
}

/// A link at the right of what is said; says whether it was clicked.
fn link(ui: &mut Ui<'_>, parts: &mut Area, (key, button): (&str, &Button<'_>)) -> bool {
    let (width, height) = ui.button_size(button);
    let rect: Rect = parts.right(width);
    let cell = place(rect, (width, height.min(rect.height)), (Align::Start, Align::Center));
    ui.button(key, cell, button).clicked
}

/// What succeeded is put away ten seconds after it was first drawn or the pointer was last on it: the moment is kept by
/// the store, which this asks to keep it, once a second while the pointer is on it; a warning and an error stay.
fn said_for(state: &SaidState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Option<Intent> {
    if state.lasts() || state.is_error {
        return None;
    }
    let now = ui.now();
    let is_on = ui.is_pointer_within(rect);
    let shown = |at| Some(Intent::Feature(FeatureIntent::SaidShown(at)));
    match state.shown_at {
        None => shown(now),
        Some(at) if is_on && now >= at + KEPT_EVERY => shown(now),
        Some(at) if is_on => {
            ui.request_frame_at(at + KEPT_EVERY);
            None
        }
        Some(at) if now >= at + SAID_FOR => Some(Intent::Feature(FeatureIntent::Dismiss)),
        Some(at) => {
            ui.request_frame_at(at + SAID_FOR);
            None
        }
    }
}
