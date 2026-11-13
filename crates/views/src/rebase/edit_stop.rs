//! `rebase.edit-stop`: the banner above the timeline while an interactive rebase is stopped right after a commit
//! marked **Edit**, or to split a commit (`split.commit`).

use model::contract::{FeatureId, Target};
use model::rebase::{RebaseStop, StopKind};
use ui::layout::{Align, Area, Insets, inset, place};
use ui::{Axis, Button, ButtonKind, Icon, Label, Rect, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::rebase as words;

/// What the banner of a stop shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EditStopState<'a> {
    /// Where the rebase stopped, and why.
    pub stop: &'a RebaseStop,
    /// Whether files are in conflict, which disables **Continue**.
    pub has_conflicts: bool,
}

/// The words of the banner.
#[must_use]
pub fn banner_words(stop: &RebaseStop) -> String {
    match stop.kind {
        StopKind::Edit => words::stopped_to_edit(&stop.subject),
        StopKind::Split => words::splitting(&stop.subject),
    }
}

/// The banner of a rebase stopped to edit or split a commit: the icon of a commit (there is no edit icon yet), what it
/// says, **Continue**, which runs `rebase.edit-stop` or `split.continue`, and **Abort**; no **Skip**, which would throw
/// away what is not committed and keep the commit anyway. **Abort** is greyed out until the operation that takes a
/// rebase back comes (`conflicts.abort-operation`).
///
/// Feature: rebase.edit-stop
pub fn edit_stop_view(state: &EditStopState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
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
    let mut parts = Area::with_gap(
        inset(rect, Insets::symmetric(spacing.large + spacing.small, 0.0)),
        spacing.medium,
    );
    ui.icon_in(parts.left(theme.sizes.icon), Icon::Commit, Tone::Accent);
    let abort = Button::secondary(words::ABORT)
        .kind(ButtonKind::Outline)
        .with_tooltip(words::ABORT_LATER)
        .disabled();
    let (width, height) = ui.button_size(&abort);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    let _ = ui.button("abort the stop", cell, &abort);
    let tooltip = if state.has_conflicts {
        words::RESOLVE_FIRST
    } else {
        words::CONTINUE_TOOLTIP
    };
    let mut continue_button = Button::primary(words::CONTINUE).with_tooltip(tooltip);
    if state.has_conflicts {
        continue_button = continue_button.disabled();
    }
    let (width, height) = ui.button_size(&continue_button);
    let cell = place(parts.right(width), (width, height), (Align::Start, Align::Center));
    let clicked = ui.button("continue the stop", cell, &continue_button).clicked;
    let said = banner_words(state.stop);
    let _ = ui.label(parts.rest(), &Label::new(&said));
    if !clicked {
        return Vec::new();
    }
    let feature = match state.stop.kind {
        StopKind::Edit => "rebase.edit-stop",
        StopKind::Split => "split.continue",
    };
    vec![Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head))]
}
