//! The first button of the top row of the timeline, which puts the side column away and brings it back, and in the
//! narrow layout opens it over the timeline (`graph.toggle-references`).

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
