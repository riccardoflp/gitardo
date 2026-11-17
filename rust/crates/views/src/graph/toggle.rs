//! The first button of the top row of the timeline and of the title row of every page, which puts the side column away
//! and brings it back, and in the narrow layout opens it over the timeline or the page (`graph.toggle-references`,
//! `repo.side-column`).

use ui::layout::Area;
use ui::{Button, ButtonKind, Ui};

use super::intent::GraphIntent;
use crate::icons;
use crate::intent::Intent;
use crate::text::header as words;

/// Whether the side column is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferencesShown {
    /// Whether it is shown, beside the timeline or over it, rather than put away.
    pub is_shown: bool,
    /// Whether the window is narrow, where the side column opens over the timeline rather than beside it.
    pub is_narrow: bool,
}

impl ReferencesShown {
    /// Whether the side column is shown where it is: beside what is shown while the window is wide and it is not put
    /// away, over the timeline or the page while the window is narrow and it was opened over it.
    #[must_use]
    pub fn of(place: crate::SidePlace, is_narrow: bool) -> ReferencesShown {
        ReferencesShown {
            is_shown: place.is_shown(is_narrow),
            is_narrow,
        }
    }
}

/// The height of the top row of the timeline and of the title row of a page: a control and its margins, so that the
/// button of the side column is at the same place in both.
pub(crate) fn top_row_height(ui: &Ui<'_>) -> f32 {
    let sizes = ui.theme().sizes;
    sizes.control_height + 2.0 * sizes.spacing.medium
}

/// The button of the side column, at the start of a row of the height of [`top_row_height`] whose parts start at its
/// left margin: the same in the top row of the timeline and in the title row of every page.
pub(crate) fn side_button(side: ReferencesShown, parts: &mut Area, ui: &mut Ui<'_>) -> Vec<Intent> {
    let width = ui.theme().sizes.control_height;
    let cell = crate::line::control_in(ui, parts.left(width), width);
    ui.clip(cell, |ui| toggle_references_view(&side, ui))
}

/// The button over the rectangle its caller clipped it to, pressed while the side column is shown: a click puts it
/// away or brings it back, and in the narrow layout opens it over the timeline or closes it.
///
/// Feature: graph.toggle-references
pub fn toggle_references_view(state: &ReferencesShown, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let tooltip = if state.is_shown {
        words::HIDE_REFERENCES
    } else {
        words::SHOW_REFERENCES
    };
    let kind = if state.is_shown {
        ButtonKind::Outline
    } else {
        ButtonKind::Icon
    };
    let button = Button::icon(icons::BRANCH, tooltip).kind(kind);
    if !ui.button("toggle references", rect, &button).clicked {
        return Vec::new();
    }
    let intent = match (state.is_narrow, state.is_shown) {
        (false, _) => GraphIntent::ToggleReferences,
        (true, true) => GraphIntent::CloseSideColumn,
        (true, false) => GraphIntent::OpenSideColumn,
    };
    vec![Intent::Graph(intent)]
}
