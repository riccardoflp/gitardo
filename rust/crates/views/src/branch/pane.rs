//! The branch rail as one place (`branch.pane`): its top, which names the branch checked out and has **Create branch**
//! and **Compare**, over the lists of `refs.tree`.

use std::time::Duration;

use model::contract::{FeatureId, Target};
use model::state::RepositoryState;
use ui::layout::{Area, Insets, inset};
use ui::{Axis, Button, ButtonKind, Icon, Label, Rect, TextKind, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::graph::HoverTarget;
use crate::intent::Intent;
use crate::line::is_narrow;
use crate::refs::{ReferencesState, rail_view};
use crate::text::header::detached_at;
use crate::text::rail as words;

/// The branch rail, over the rectangle its caller clipped it to: at its top the branch checked out, **Create branch**,
/// which makes a branch at `HEAD` (`branch.create`), and **Compare** (`compare.choose`); under it the filter box, the
/// line of the branches shown and the lists of `refs.tree`. In the narrow layout there is no top, as there is no box.
///
/// Feature: branch.pane
pub fn pane(state: &ReferencesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    pane_view(state, ui).0
}

/// The branch rail with its top, and the reference the pointer rests on with since when (`graph.hover-history`).
pub(crate) fn pane_view(
    state: &ReferencesState<'_>,
    ui: &mut Ui<'_>,
) -> (Vec<Intent>, Option<(HoverTarget, Duration)>) {
    let rect = ui.current_clip();
    if is_narrow(ui) {
        return rail_view(state, ui);
    }
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let height = spacing.large + 2.0 * theme.sizes.control_height + spacing.small;
    let mut parts = Area::new(rect);
    let top = parts.top(height);
    ui.fill_rect(top, theme.colors.rail_background);
    let edge = Rect::new(top.right() - theme.sizes.border, top.y, theme.sizes.border, top.height);
    ui.separator(edge, Axis::Vertical);
    let mut intents = ui.clip(top, |ui| top_view(state.repository, ui));
    let (rail_intents, hover) = ui.clip(parts.rest(), |ui| rail_view(state, ui));
    intents.extend(rail_intents);
    (intents, hover)
}

/// The name of the branch checked out, or the commit of a detached `HEAD`, over the two buttons; neither can be used
/// before the first commit.
fn top_view(repository: &RepositoryState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let inner = inset(
        rect,
        Insets {
            bottom: 0.0,
            ..Insets::all(spacing.large)
        },
    );
    let mut rows = Area::with_gap(inner, spacing.small);
    let mut name_line = Area::with_gap(rows.top(theme.sizes.control_height), spacing.small);
    ui.icon_in(name_line.left(theme.sizes.icon), Icon::Branch, Tone::Muted);
    let name = checked_out(repository);
    let _ = ui.label(name_line.rest(), &Label::new(&name).kind(TextKind::Strong));
    let mut buttons = Area::with_gap(rows.top(theme.sizes.control_height), spacing.small);
    let can_run = repository.has_commit();
    let available = |button: Button<'static>| if can_run { button } else { button.disabled() };
    let compare = available(Button::secondary(words::COMPARE));
    let create = Button::secondary(words::CREATE_BRANCH).with_icon(Icon::Plus);
    let (compare_width, _) = ui.button_size(&compare);
    let (create_width, _) = ui.button_size(&create);
    let width = buttons.rest().width;
    // The words of Create branch go when both buttons do not fit beside each other.
    let create = if create_width + compare_width + spacing.small <= width {
        create
    } else {
        Button::icon(Icon::Plus, words::CREATE_BRANCH).kind(ButtonKind::Secondary)
    };
    let create = available(create);
    let (create_width, _) = ui.button_size(&create);
    let mut intents = Vec::new();
    let run = |feature: &'static str| Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head));
    if ui
        .button("rail create branch", buttons.left(create_width), &create)
        .clicked
    {
        intents.push(run("branch.create"));
    }
    if ui.button("rail compare", buttons.left(compare_width), &compare).clicked {
        intents.push(run("compare.choose"));
    }
    intents
}

/// The branch checked out, or `detached at <short hash>`.
fn checked_out(repository: &RepositoryState) -> String {
    match (repository.checked_out_branch(), &repository.head.commit) {
        (Some(branch), _) => branch.to_owned(),
        (None, Some(commit)) => detached_at(commit.short()),
        (None, None) => String::new(),
    }
}
