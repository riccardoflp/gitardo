//! An operation stopped on conflicts, in the window: the banner above the commits and the Changes page
//! (`conflicts.banner`), the message when an operation stops (`conflicts.stopped`), and a file in conflict opened where
//! it can be resolved (`conflicts.open-file`).

mod open_file;
mod stopped;
#[cfg(test)]
mod tests;

pub use open_file::{ConflictsIntent, OpenAt, OpenFileState, open_file_view, open_target};
pub use stopped::{StoppedState, stopped_view};

use model::conflicts::RuleCounts;
use model::contract::{FeatureId, Target};
use model::state::Operation;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{AccessRole, Axis, Button, ButtonKind, Label, Rect, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::icons;
use crate::intent::Intent;
use crate::rebase::EditStopState;
use crate::text::conflicts as words;

/// What the banner of an operation that stopped shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BannerState<'a> {
    /// The operation in progress; none when files are in conflict without one, as after a stash.
    pub operation: Option<Operation>,
    /// How many files are in conflict.
    pub conflicted: usize,
    /// Where an interactive rebase stopped to edit or to split a commit, whose banner this is then.
    pub stop: Option<EditStopState<'a>>,
    /// How many conflicts the files have and how many need no choice, once counted (`conflicts.resolve-by-rules`).
    pub counts: Option<RuleCounts>,
    /// `conflicts.autoResolve`.
    pub auto_resolve: bool,
    /// `conflicts.ai`.
    pub ai: bool,
    /// Whether Gitardo is running something on the repository: every button is disabled.
    pub is_running: bool,
}

/// A button of the banner, as it is drawn.
#[derive(Clone, Debug, PartialEq)]
pub struct BannerButton {
    /// Its words.
    pub label: String,
    /// What its tooltip says.
    pub tooltip: Option<&'static str>,
    /// Whether it can be pressed.
    pub is_enabled: bool,
    /// What pressing it asks; none for a button whose feature is not there yet.
    pub intent: Option<Intent>,
}

impl BannerState<'_> {
    /// What the banner says, the text of a stop first.
    #[must_use]
    pub fn says(&self) -> String {
        match &self.stop {
            Some(stop) => crate::rebase::banner_words(stop.stop),
            None => words::banner(self.operation, self.conflicted),
        }
    }

    /// The buttons of the banner of an operation that stopped, from the left: **Resolve … automatically**, which opens
    /// the review of the conflicts, **Resolve with AI**, **Continue**, **Skip** and **Abort**, each when it applies.
    #[must_use]
    pub fn buttons(&self) -> Vec<BannerButton> {
        let mut buttons = Vec::new();
        let can_press = !self.is_running;
        let counts = self.counts.unwrap_or_default();
        if self.auto_resolve && counts.resolvable > 0 {
            buttons.push(BannerButton {
                label: words::resolve_automatically(counts.resolvable, counts.conflicts),
                tooltip: Some(words::RESOLVE_AUTOMATICALLY_TOOLTIP),
                is_enabled: can_press,
                intent: Some(Intent::Review(crate::review::ReviewIntent::Open(None))),
            });
        }
        let rules_leave_some = counts.conflicts == 0 || counts.resolvable < counts.conflicts;
        if self.ai && self.conflicted > 0 && rules_leave_some {
            buttons.push(BannerButton {
                label: words::RESOLVE_WITH_AI.to_owned(),
                tooltip: None,
                is_enabled: can_press,
                intent: Some(Intent::Review(crate::review::ReviewIntent::ResolveWithAi(None))),
            });
        }
        let Some(operation) = self.operation else {
            return buttons;
        };
        buttons.push(BannerButton {
            label: words::CONTINUE.to_owned(),
            tooltip: (self.conflicted > 0).then_some(words::RESOLVE_FIRST),
            is_enabled: can_press && self.conflicted == 0,
            intent: Some(run("conflicts.continue-operation")),
        });
        if operation != Operation::Merge && self.stop.is_none() {
            buttons.push(BannerButton {
                label: words::SKIP.to_owned(),
                tooltip: None,
                is_enabled: can_press,
                intent: Some(run("conflicts.skip-step")),
            });
        }
        buttons.push(BannerButton {
            label: words::ABORT.to_owned(),
            tooltip: None,
            is_enabled: can_press,
            intent: Some(run("conflicts.abort-operation")),
        });
        buttons
    }
}

/// The banner of an operation that stopped, over the rectangle its caller clipped it to: what it says, after a warning
/// icon, and its buttons; at a stop to edit or to split a commit, the banner of that stop (`rebase.edit-stop`), and
/// while `git am` is under way, the banner of the patch (`patch.am-stopped`).
///
/// Feature: conflicts.banner
pub fn banner_view(state: &BannerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    if let Some(stop) = &state.stop {
        return crate::rebase::edit_stop_view(stop, ui);
    }
    if state.operation == Some(Operation::Patch) {
        return crate::patch::am_stopped(state, ui);
    }
    operation_banner(state, ui)
}

/// The banner of an operation that stopped: what it says, after a warning icon, and its buttons.
pub(crate) fn operation_banner(state: &BannerState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let line = Rect::new(
        rect.x,
        rect.bottom() - theme.sizes.border,
        rect.width,
        theme.sizes.border,
    );
    ui.separator(line, Axis::Horizontal);
    let mut intents = Vec::new();
    ui.region("conflicts banner", (rect, AccessRole::Group, words::BANNER), |ui| {
        let mut parts = Area::with_gap(
            inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
            spacing.medium,
        );
        ui.icon_in(parts.left(theme.sizes.icon), icons::WARNING, Tone::Warning);
        for button in state.buttons().into_iter().rev() {
            let mut drawn = if button.label == words::CONTINUE {
                Button::primary(&button.label)
            } else {
                Button::secondary(&button.label).kind(ButtonKind::Outline)
            };
            if !button.is_enabled {
                drawn = drawn.disabled();
            }
            let (width, height) = ui.button_size(&drawn);
            let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
            let response = ui.button(("conflicts banner", button.label.as_str()), cell, &drawn);
            if let Some(tooltip) = button.tooltip {
                ui.tooltip(&response, tooltip);
            }
            if response.clicked {
                intents.extend(button.intent);
            }
        }
        let said = state.says();
        let _ = ui.label(parts.rest(), &Label::new(&said));
    });
    intents
}

/// The intent that runs an operation of the conflicts on what is checked out.
fn run(feature: &'static str) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head))
}
