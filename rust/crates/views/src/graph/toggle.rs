//! The first button of the header, which puts the branch rail away and brings it back (`graph.toggle-references`).

use ui::{Button, ButtonKind, Icon, Ui};

use super::intent::GraphIntent;
use crate::intent::Intent;
use crate::text::header as words;

/// Whether the branch rail is shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReferencesShown {
    /// Whether it is shown, rather than put away.
    pub is_shown: bool,
}

/// The button over the rectangle its caller clipped it to, pressed while the branch rail is shown.
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
    let button = Button::icon(Icon::Branch, tooltip).kind(kind);
    if ui.button("toggle references", rect, &button).clicked {
        vec![Intent::Graph(GraphIntent::ToggleReferences)]
    } else {
        Vec::new()
    }
}
