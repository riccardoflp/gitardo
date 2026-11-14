//! What the search found, at the end of its box (`search.history`): a spinning icon while git looks, **Searching…**
//! while the list of a search in the changes is read, then the count and the buttons that step through the matches.

use model::search::Direction;
use ui::layout::{Align, Area};
use ui::{Button, ButtonKind, Icon, Label, Rect, Sense, TextKind, Tone, Ui};

use super::SearchIntent;
use crate::intent::Intent;
use crate::text::search as words;

/// What a search found, as the end of its box says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct HistorySearch {
    /// Whether git still looks.
    pub is_looking: bool,
    /// Whether the list of a search in the changes is read.
    pub is_listing: bool,
    /// What was found, once something was asked.
    pub count: Option<MatchCount>,
}

/// How many matches there are, and which of them is selected.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MatchCount {
    /// The place of the commit selected among the matches, from 1, when it is one of them.
    pub selected: Option<usize>,
    /// How many matches were counted.
    pub total: usize,
    /// Whether the search stopped counting.
    pub has_more: bool,
}

impl HistorySearch {
    /// The words of the count, when there is one to say.
    #[must_use]
    pub fn count_words(&self) -> Option<String> {
        if self.is_looking || self.is_listing {
            return None;
        }
        self.count
            .map(|count| words::count(count.selected, count.total, count.has_more))
    }

    /// Whether there are matches to step through.
    fn can_step(&self) -> bool {
        !self.is_looking && self.count.is_some_and(|count| count.total > 0)
    }
}

/// How wide the end of a box is for what a search found.
pub(crate) fn history_width(state: &HistorySearch, ui: &Ui<'_>) -> f32 {
    let theme = ui.theme();
    let spacing = theme.sizes.spacing;
    let side = theme.sizes.control_height;
    let words = if state.is_listing {
        Some(words::SEARCHING.to_owned())
    } else {
        state.count_words()
    };
    let text = words.map_or(0.0, |words| {
        ui.measure(&words, ui.text_style(TextKind::Detail)) + spacing.medium
    });
    let spinner = if state.is_looking || state.is_listing {
        theme.sizes.icon + spacing.medium
    } else {
        0.0
    };
    let buttons = if state.can_step() { 2.0 * side } else { 0.0 };
    text + spinner + buttons
}

/// The end of the box for what a search found, over the rectangle its caller clipped it to; the buttons step through
/// the matches.
///
/// Feature: search.history
pub fn history_view(state: &HistorySearch, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let side = theme.sizes.control_height;
    let mut area = Area::with_gap(rect, 0.0);
    let mut intents = Vec::new();
    if state.can_step() {
        let next = Button::icon(Icon::ArrowDown, words::NEXT_MATCH).kind(ButtonKind::Icon);
        if ui.button("next match", area.right(side), &next).clicked {
            intents.push(Intent::Search(SearchIntent::Step(Direction::Next)));
        }
        let previous = Button::icon(Icon::ArrowUp, words::PREVIOUS_MATCH).kind(ButtonKind::Icon);
        if ui.button("previous match", area.right(side), &previous).clicked {
            intents.push(Intent::Search(SearchIntent::Step(Direction::Previous)));
        }
    }
    let said = if state.is_listing {
        Some(words::SEARCHING.to_owned())
    } else {
        state.count_words()
    };
    if let Some(said) = said {
        let width = ui.measure(&said, ui.text_style(TextKind::Detail)) + spacing.medium;
        let cell = area.right(width);
        let label = Label::new(&said)
            .kind(TextKind::Detail)
            .tone(Tone::Muted)
            .align(Align::Center);
        let _ = ui.label(cell, &label);
        if state.count.is_some_and(|count| count.has_more) && !state.is_listing {
            let id = ui.id("first matches counted");
            let response = ui.interact(id, cell, Sense::HOVER);
            ui.tooltip(&response, words::FIRST_MATCHES_COUNTED);
        }
    }
    if state.is_looking || state.is_listing {
        let cell = area.right(theme.sizes.icon + spacing.medium);
        ui.spinner(Rect::new(cell.x, rect.y, cell.width, rect.height));
    }
    intents
}
