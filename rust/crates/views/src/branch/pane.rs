//! The branch rail as one place (`branch.pane`): the lists of `refs.tree`, and its buttons, which the heading of
//! **Branches** in the side column has on its right: **Create branch**, **Compare** and the button of the rail's own
//! filter.

use std::time::Duration;

use model::contract::{FeatureId, Target};
use ui::layout::Area;
use ui::{Button, ButtonKind, Rect, Ui};

use crate::feature::FeatureIntent;
use crate::graph::HoverTarget;
use crate::icons;
use crate::intent::Intent;
use crate::refs::{ReferencesState, list_filter_view, rail_view};
use crate::text::rail as words;

/// The branch rail, over the rectangle its caller clipped it to: the filter box, the line of what the rail lists and
/// the lists of `refs.tree`. Its buttons are in the heading of **Branches** ([`pane_buttons`]); narrowed to its icons,
/// the button of the filter is at the top of the icons.
///
/// Feature: branch.pane
pub fn pane(state: &ReferencesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    pane_view(state, ui).0
}

/// The branch rail, and the reference the pointer rests on with since when (`graph.hover-history`).
pub(crate) fn pane_view(
    state: &ReferencesState<'_>,
    ui: &mut Ui<'_>,
) -> (Vec<Intent>, Option<(HoverTarget, Duration)>) {
    rail_view(state, ui)
}

/// How wide the buttons of the rail are, side by side, square in a heading of a height.
pub(crate) fn buttons_width(ui: &Ui<'_>, height: f32) -> f32 {
    3.0 * height + 2.0 * ui.theme().sizes.spacing.tiny
}

/// The buttons of the rail, from the right of a rectangle: the button of the rail's own filter (`refs.list-filter`),
/// **Compare** (`compare.choose`) and **Create branch**, which makes a branch at `HEAD` (`branch.create`), each an icon
/// with its name in its tooltip; neither of the last two can be used before the first commit.
pub(crate) fn pane_buttons(state: &ReferencesState<'_>, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let side = rect.height;
    let mut parts = Area::with_gap(rect, ui.theme().sizes.spacing.tiny);
    let filter = parts.right(side);
    let compare_cell = parts.right(side);
    let create_cell = parts.right(side);
    let mut intents = ui.clip(filter, |ui| list_filter_view(&state.list_filter, ui));
    let can_run = state.repository.has_commit();
    let available = |button: Button<'static>| if can_run { button } else { button.disabled() };
    let create = available(Button::icon(icons::CREATE_BRANCH, words::CREATE_BRANCH).kind(ButtonKind::Icon));
    let compare = available(Button::icon(icons::COMPARE, words::COMPARE).kind(ButtonKind::Icon));
    let run = |feature: &'static str| Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head));
    if ui.button("rail create branch", create_cell, &create).clicked {
        intents.push(run("branch.create"));
    }
    if ui.button("rail compare", compare_cell, &compare).clicked {
        intents.push(run("compare.choose"));
    }
    intents
}
