//! Where the history of a shallow clone ends (`graph.shallow-end`): the last row of the timeline says so, with the two
//! fetches that bring more of it, so that the last commit listed is not taken for the first commit of the project.

use model::contract::{FeatureId, Target};
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Label, Rect, TextKind, Tone, Ui};

use crate::feature::FeatureIntent;
use crate::intent::Intent;
use crate::text::timeline as words;

/// Where the last row of a shallow history is drawn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShallowEndState {
    /// The row, across the whole list.
    pub row: Rect,
    /// The width of the graph at its left, which the words start after.
    pub graph_width: f32,
}

/// The intent of a button that fetches more of the history: `sync.deepen` or `sync.unshallow`.
#[must_use]
pub fn fetch_more(feature: &'static str) -> Intent {
    Intent::Feature(FeatureIntent::Run(FeatureId(feature), Target::Head))
}

/// The last row of the timeline of a shallow clone, once its last page is read: **The history ends here: this is a
/// shallow clone.**, then **Load more history** (`sync.deepen`) and **Load the whole history** (`sync.unshallow`).
///
/// Feature: graph.shallow-end
pub fn shallow_end_view(state: &ShallowEndState, ui: &mut Ui<'_>) -> Vec<Intent> {
    let theme = *ui.theme();
    let gap = theme.sizes.spacing.medium;
    let row = state.row;
    let rect = Rect::new(
        row.x + state.graph_width + theme.sizes.spacing.large,
        row.y,
        (row.width - state.graph_width - 2.0 * theme.sizes.spacing.large).max(0.0),
        row.height,
    );
    let (said, buttons) = if rect.height >= 2.0 * theme.sizes.row_height {
        let half = rect.height * 0.5;
        (
            Rect::new(rect.x, rect.y, rect.width, half),
            Rect::new(rect.x, rect.y + half, rect.width, half),
        )
    } else {
        let width = ui.measure(words::SHALLOW_END, ui.text_style(TextKind::Body)) + gap;
        let said = Rect::new(rect.x, rect.y, width.min(rect.width), rect.height);
        (
            said,
            Rect::new(said.right(), rect.y, (rect.width - said.width).max(0.0), rect.height),
        )
    };
    let _ = ui.label(said, &Label::new(words::SHALLOW_END).tone(Tone::Muted));
    let mut area = Area::with_gap(buttons, gap);
    let mut intents = Vec::new();
    for (id, label, feature) in [
        ("load more history", words::LOAD_MORE_HISTORY, "sync.deepen"),
        ("load the whole history", words::LOAD_WHOLE_HISTORY, "sync.unshallow"),
    ] {
        let button = Button::secondary(label).kind(ButtonKind::Icon);
        let (width, height) = ui.button_size(&button);
        let size = (width, height.min(buttons.height));
        let cell = place(area.left(width), size, (Align::Start, Align::Center));
        if ui.button(id, cell, &button).clicked {
            intents.push(fetch_more(feature));
        }
    }
    intents
}
