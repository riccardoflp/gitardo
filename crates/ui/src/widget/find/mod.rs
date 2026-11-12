//! Find in page: Ctrl+F opens a bar at the top right of a page, which finds what is typed in it in the text the page
//! drew in this frame, and in every row of the lists that search their rows themselves, in sight or not. The matches
//! are lit; Enter and Shift+Enter, or F3 and Shift+F3, go to the next and the previous one, scrolling a list to it;
//! the bar counts them, "3 of 12", has a switch for the case of letters, and Escape closes it.

mod bar;
mod matches;
mod search;

use std::hash::Hash;

use render::{Border, Rect};

use crate::frame::Ui;
use crate::id::WidgetId;
use crate::input::{Key, KeyEvent, Modifiers, NamedKey};
use matches::{PageMatches, PageSearch, Place, match_rects, page_matches};
pub use search::MatchCase;
pub(crate) use search::{Recording, RowMatches, RowOwner, RowSearch, SearchedList};

/// What find in page remembers between frames.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct FindState {
    /// Whether the bar is open.
    pub(crate) is_open: bool,
    /// What is looked for.
    pub(crate) query: String,
    /// Whether the case of letters matters.
    pub(crate) case: MatchCase,
    /// The number of the match that is current, among all of them.
    pub(crate) current: usize,
    /// Whether the current match must be brought into sight, because it just changed.
    pub(crate) must_reveal: bool,
}

impl FindState {
    /// Whether there is something to find: the bar is open and something is typed.
    pub(crate) fn is_searching(&self) -> bool {
        self.is_open && !self.query.is_empty()
    }
}

/// What find in page tells its view.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FindResponse {
    /// Whether the bar is open.
    pub is_open: bool,
    /// How many matches there are.
    pub match_count: usize,
    /// The number of the current match, from zero.
    pub current: Option<usize>,
}

/// Where the bar goes to among the matches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Step {
    /// The next match.
    Next,
    /// The match before.
    Previous,
}

/// The rows of a list that can be searched for find in page, in sight or not: the text of each row by its number,
/// and a number the view changes whenever the rows change, so that they are searched again.
#[derive(Clone, Copy)]
pub struct ListSearch<'rows> {
    /// The text of a row, as find in page should see it.
    pub row_text: &'rows dyn Fn(usize) -> String,
    /// The version of the rows.
    pub generation: u64,
}

impl std::fmt::Debug for ListSearch<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ListSearch")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl Ui<'_> {
    /// Declares find in page over a page, after everything the page draws: opens the bar on Ctrl+F, lights the
    /// matches, and draws the bar while it is open.
    pub fn find_in_page(&mut self, key: impl Hash, page: Rect) -> FindResponse {
        let id = self.id(key);
        let input = id.with("query");
        self.open_find_on_shortcut(input);
        if !self.memory_view().find.is_open {
            return FindResponse::default();
        }
        let is_new = self.declare_overlay(id, &[input]);
        if is_new {
            self.request_focus(input);
        }
        let step = self.find_keys(input);
        if !self.memory_view().find.is_open {
            return FindResponse::default();
        }
        let state = self.memory_view().find.clone();
        let search = PageSearch {
            page,
            query: &state.query,
            case: state.case,
        };
        let found = page_matches(&self.recording, &self.memory_view().list_searches, search);
        let current = self.step_to(step, found.total);
        let place = (found.total > 0)
            .then(|| found.place(current, &self.memory_view().list_searches))
            .flatten();
        self.light_matches(&found, place, search);
        self.reveal_match(place);
        self.recording.hold(true);
        self.find_bar(id, page, &found);
        self.recording.hold(false);
        let is_open = self.memory_view().find.is_open;
        FindResponse {
            is_open,
            match_count: found.total,
            current: (is_open && found.total > 0).then_some(current),
        }
    }

    /// Opens the bar on Ctrl+F when no widget used it, or gives its input box the focus with the text selected when it
    /// is open already.
    fn open_find_on_shortcut(&mut self, input: WidgetId) {
        let shortcut = KeyEvent::press(Key::Character('f'), Modifiers::CONTROL);
        let pressed = self
            .unclaimed_keys()
            .into_iter()
            .any(|key| key.key == shortcut.key && key.modifiers == shortcut.modifiers);
        if !pressed {
            return;
        }
        self.claim_any_key(shortcut);
        self.open_find_bar(input);
    }

    /// Opens the find bar of a page, as Ctrl+F does, for a menu item or a button that opens it.
    pub fn open_find(&mut self, key: impl Hash) {
        let input = self.id(key).with("query");
        self.open_find_bar(input);
    }

    /// Opens the find bar, its input box with the focus and what it holds selected.
    fn open_find_bar(&mut self, input: WidgetId) {
        let query = self.memory_view().find.query.clone();
        self.memory().find.is_open = true;
        self.memory().set_input_text(input, &query, (0, query.len()));
        self.request_focus(input);
    }

    /// The keys of the bar: Enter and Shift+Enter in its input box, F3 and Shift+F3 anywhere, step through the
    /// matches; Escape in the input box closes it.
    fn find_keys(&mut self, input: WidgetId) -> Option<Step> {
        let mut step = None;
        for key in self.unclaimed_keys() {
            let is_in_box = self.unclaimed_keys_of(input).contains(&key);
            let wanted = match (key.key, key.modifiers) {
                (Key::Named(NamedKey::Enter), Modifiers::NONE) if is_in_box => Some(Step::Next),
                (Key::Named(NamedKey::Enter), Modifiers::SHIFT) if is_in_box => Some(Step::Previous),
                (Key::Named(NamedKey::Function(3)), Modifiers::NONE) => Some(Step::Next),
                (Key::Named(NamedKey::Function(3)), Modifiers::SHIFT) => Some(Step::Previous),
                (Key::Named(NamedKey::Escape), Modifiers::NONE) if is_in_box => {
                    self.memory().find.is_open = false;
                    None
                }
                _ => continue,
            };
            self.claim_any_key(key);
            step = wanted.or(step);
        }
        step
    }

    /// Moves the current match a step, round from the last to the first, and returns it, inside the matches there
    /// are.
    fn step_to(&mut self, step: Option<Step>, total: usize) -> usize {
        let find = &mut self.memory().find;
        if total == 0 {
            find.current = 0;
            return 0;
        }
        find.current = match step {
            Some(Step::Next) => (find.current + 1) % total,
            Some(Step::Previous) => (find.current + total - 1) % total,
            None => find.current.min(total - 1),
        };
        find.must_reveal |= step.is_some();
        find.current
    }

    /// Lights every match in sight, and the current one more.
    fn light_matches(&mut self, found: &PageMatches, current: Option<Place>, search: PageSearch<'_>) {
        if found.total == 0 {
            return;
        }
        let theme = *self.theme();
        let texts = std::mem::take(&mut self.recording.texts);
        let mut row_counts: Vec<(RowOwner, usize)> = Vec::new();
        for (index, text) in texts.iter().enumerate() {
            let rects = match_rects(self.fonts(), text, search.query, search.case);
            let before = text
                .owner
                .map_or(0, |owner| earlier_in_row(&mut row_counts, owner, rects.len()));
            for (occurrence, rect) in rects.into_iter().enumerate() {
                let Some(rect) = rect else {
                    continue;
                };
                let is_current = match (current, text.owner) {
                    (
                        Some(Place::Text {
                            text: at,
                            occurrence: which,
                        }),
                        _,
                    ) => at == index && which == occurrence,
                    (
                        Some(Place::Row {
                            list,
                            row,
                            occurrence: which,
                        }),
                        Some(owner),
                    ) => owner.list == list && owner.row == row && which == before + occurrence,
                    _ => false,
                };
                self.layer(text.layer, |ui| {
                    ui.clip(text.clip, |ui| {
                        ui.fill_rect(rect, theme.colors.find_match);
                        if is_current {
                            let border = Border {
                                width: theme.sizes.focus_ring,
                                radius: 0.0,
                                color: theme.colors.focus_ring,
                            };
                            ui.border(rect.inset(-theme.sizes.focus_ring), border);
                        }
                    });
                });
            }
        }
        self.recording.texts = texts;
    }

    /// Scrolls the list of the current match to it, when the match just changed and is in a row out of sight.
    fn reveal_match(&mut self, place: Option<Place>) {
        if !self.memory_view().find.must_reveal || place.is_none() {
            return;
        }
        self.memory().find.must_reveal = false;
        let Some(Place::Row { list, row, .. }) = place else {
            return;
        };
        let Some(searched) = self
            .recording
            .lists
            .iter()
            .find(|searched| searched.list == list)
            .copied()
        else {
            return;
        };
        let height = f64::from(searched.row_height);
        let start = crate::number::wide_of_count(row) * height;
        if let Some(state) = self.memory().lists.get_mut(&list) {
            let before = state.scroll.target();
            state
                .scroll
                .reveal((start, start + height), f64::from(searched.rect.height));
            if (state.scroll.target() - before).abs() > f64::EPSILON {
                let now = self.now();
                self.request_frame_at(now);
            }
        }
    }
}

/// How many matches the texts drawn before in the same row of a list had, counting those of this text for the next.
fn earlier_in_row(row_counts: &mut Vec<(RowOwner, usize)>, owner: RowOwner, count: usize) -> usize {
    if let Some((_, seen)) = row_counts.iter_mut().find(|(seen, _)| *seen == owner) {
        return std::mem::replace(seen, *seen + count);
    }
    row_counts.push((owner, count));
    0
}
