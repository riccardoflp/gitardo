//! `conflicts.stopped`: the message, in place of what an operation says once it ended, when it stopped on conflicts.

use model::conflicts::RuleCounts;
use model::contract::{FeatureId, Target};
use model::state::Operation;
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Button, ButtonKind, Icon, Label, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::conflicts as words;
use crate::text::feature as feature_words;

/// What the message says and offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoppedState<'a> {
    /// The title of the operation that stopped: *Merge feature into main*.
    pub title: &'a str,
    /// How many files are in conflict.
    pub conflicted: usize,
    /// The operation left to continue or abort; none when there is none, as after a stash.
    pub operation: Option<Operation>,
    /// How many conflicts the files have and how many need no choice, once counted.
    pub counts: Option<RuleCounts>,
    /// `conflicts.autoResolve`.
    pub auto_resolve: bool,
    /// `conflicts.ai`.
    pub ai: bool,
}

impl StoppedState<'_> {
    /// What the message says.
    #[must_use]
    pub fn says(&self) -> String {
        let noun = self.operation.map(words::operation_noun);
        let mut said = words::stopped(self.title, self.conflicted, noun);
        if let Some(counts) = self.resolvable() {
            said.push(' ');
            said.push_str(&words::can_resolve(counts.resolvable, counts.conflicts));
        }
        said
    }

    /// The counts, when the rules resolve some conflicts and resolving them is switched on.
    fn resolvable(&self) -> Option<RuleCounts> {
        self.counts.filter(|counts| self.auto_resolve && counts.resolvable > 0)
    }

    /// Its buttons, from the left: **Resolve automatically** when the rules resolve any, **Resolve with AI** when it is
    /// on and the rules leave some, and **Show log**; none of them asks anything when its feature is not there yet.
    #[must_use]
    pub fn buttons(&self) -> Vec<(&'static str, Option<Intent>)> {
        let mut buttons = Vec::new();
        if self.resolvable().is_some() {
            let resolve = FeatureIntent::Run(FeatureId("conflicts.resolve-automatically"), Target::Head);
            buttons.push((words::RESOLVE_AUTOMATICALLY, Some(Intent::Feature(resolve))));
        }
        let counts = self.counts.unwrap_or_default();
        if self.ai && (counts.conflicts == 0 || counts.resolvable < counts.conflicts) {
            let resolve = Intent::Review(crate::review::ReviewIntent::ResolveWithAi(None));
            buttons.push((words::RESOLVE_WITH_AI, Some(resolve)));
        }
        buttons.push((words::SHOW_LOG, Some(Intent::ShowCommandLog)));
        buttons
    }
}

/// The message when an operation stopped on conflicts, over the rectangle its caller clipped it to: a warning icon,
/// what to do next, its buttons and **Dismiss**.
///
/// Feature: conflicts.stopped
pub fn stopped_view(state: &StoppedState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    ui.fill_rect(rect, theme.colors.card_background);
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
        spacing.medium,
    );
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Warning, Tone::Warning);
    let mut intents = Vec::new();
    let dismiss = (feature_words::DISMISS, Some(Intent::Feature(FeatureIntent::Dismiss)));
    for (label, intent) in std::iter::once(dismiss).chain(state.buttons().into_iter().rev()) {
        let mut button = Button::secondary(label).kind(ButtonKind::Outline);
        if intent.is_none() {
            button = button.disabled();
        }
        let (width, height) = ui.button_size(&button);
        let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
        if ui.button(("stopped", label), cell, &button).clicked {
            intents.extend(intent);
        }
    }
    let said = state.says();
    let _ = ui.label(parts.rest(), &Label::new(&said));
    intents
}
