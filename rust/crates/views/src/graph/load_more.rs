//! Asking for the next page of commits while scrolling (`graph.load-more`), long before the end of those loaded.

use ui::layout::{Align, Area, place};
use ui::{Rect, Tone, Ui};

use super::intent::GraphIntent;
use crate::intent::Intent;
use crate::text::timeline as words;

/// How close to the end of the commits loaded the last row drawn is when the next page is asked for.
pub const AHEAD: usize = 500;

/// Where the list stands against the commits loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoadMoreState {
    /// The commits loaded, the commits of folded merges and the stashes counted too.
    pub loaded: usize,
    /// The last commit drawn, by its place among those loaded; none while nothing is drawn.
    pub last_drawn: Option<usize>,
    /// Whether the history has more commits than those loaded.
    pub has_more: bool,
    /// Whether the next page is being read.
    pub is_reading: bool,
    /// The commits loaded when the next page was asked for last.
    pub asked_at: Option<usize>,
}

/// Whether the next page is wanted now: there are more, none is being read, it was not asked for at this number of
/// commits loaded, and the last row drawn is [`AHEAD`] rows or fewer from the end of them.
#[must_use]
pub fn wants_more(state: &LoadMoreState) -> bool {
    let Some(last) = state.last_drawn else {
        return false;
    };
    state.has_more
        && !state.is_reading
        && state.asked_at != Some(state.loaded)
        && state.loaded.saturating_sub(last) <= AHEAD
}

/// Asks for the next page when the list is scrolled near the end of the commits loaded; the bar and the last row that
/// say it is read are drawn by the timeline.
///
/// Feature: graph.load-more
pub fn load_more_view(state: &LoadMoreState, _ui: &mut Ui<'_>) -> Vec<Intent> {
    if wants_more(state) {
        vec![Intent::Graph(GraphIntent::LoadMore { loaded: state.loaded })]
    } else {
        Vec::new()
    }
}

/// The last row while more commits are to be read: **Loading more commits…** with a turning icon.
pub(super) fn loading_row(ui: &mut Ui<'_>, graph_width: f32, row: &ui::Row) {
    let theme = *ui.theme();
    let rect = Rect::new(
        row.rect.x + graph_width + theme.sizes.spacing.large,
        row.rect.y,
        (row.rect.width - graph_width - 2.0 * theme.sizes.spacing.large).max(0.0),
        row.rect.height,
    );
    let mut area = Area::with_gap(rect, theme.sizes.spacing.medium);
    let icon = area.left(theme.sizes.icon);
    let spinner = place(
        icon,
        (theme.sizes.icon, theme.sizes.icon),
        (Align::Center, Align::Center),
    );
    ui.spinner(spinner);
    let _ = ui.label(area.rest(), &ui::Label::new(words::LOADING_MORE).tone(Tone::Muted));
}
