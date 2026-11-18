//! The Search page (`file-search.page`, decision D73): the box of what is looked for with its switches, the modes and
//! the scopes, the patterns that narrow the files, the results found as it is typed, and the preview of the result
//! under the keyboard.

mod results;
mod top;

use model::file_search::{Found, Mode, Place, ResultRow, Scope, Search};
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Key, Label, Modifiers, Tone, Ui};

use crate::intent::Intent;
use crate::text::file_search as words;

/// What the Search page shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SearchPageState<'a> {
    /// What is looked for, and where.
    pub search: &'a Search,
    /// What was found.
    pub found: &'a Found,
    /// The rows of the results.
    pub rows: &'a [ResultRow],
    /// The row under the keyboard.
    pub lit: Option<usize>,
    /// How far the search is.
    pub status: Status<'a>,
    /// What git said of a regular expression it could not read.
    pub refused: Option<&'a str>,
    /// A number that changes each time the box takes the keyboard.
    pub focus: u64,
    /// A number that changes each time the row under the keyboard is to be brought into sight.
    pub reveal: u64,
}

/// How far a search is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status<'a> {
    /// The files are being listed.
    Reading,
    /// Git could not list the files: what it said.
    Failed(&'a str),
    /// The text is being searched: the lines come as git finds them.
    Searching,
    /// Everything found is listed.
    Done,
}

/// A switch of the page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Switch {
    /// **Match case**.
    MatchCase,
    /// **Whole word**.
    WholeWord,
    /// **Regular expression**.
    Regex,
    /// **.gitignore**.
    Gitignore,
    /// **Hidden**.
    Hidden,
}

/// Something asked of the Search page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FileSearchIntent {
    /// Show the page, from the bar of views, the key or Quick search (`file-search.open`).
    Show,
    /// **Search here** on a folder or a file of the tree of the Files page; none for its top row (`file-search.open`).
    Here(Option<Place>),
    /// What is typed in the box.
    Type(String),
    /// A switch turned.
    Turn(Switch),
    /// A mode chosen.
    Mode(Mode),
    /// A scope chosen.
    Scope(Scope),
    /// The scope **Search here** named taken away.
    DropScope,
    /// What is typed in **Include**.
    Include(String),
    /// What is typed in **Exclude**.
    Exclude(String),
    /// A row of the results comes under the keyboard.
    Light(usize),
    /// A row of the results is opened, by `Enter` or a double click.
    Open(usize),
}

/// The Search page over the rectangle its caller clipped it to: the box of what is looked for and its switches, the
/// modes and the scopes, **Include** and **Exclude**, then the results with the line that counts them.
///
/// Feature: file-search.page
pub fn page_view(state: &SearchPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let theme = *ui.theme();
    let spacing = theme.sizes.spacing;
    let mut asked = Vec::new();
    ui.region("search page", (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::with_gap(inset(rect, Insets::all(spacing.large)), spacing.medium);
        let control = theme.sizes.control_height;
        let query = area.top(control);
        let has_room = top::has_room_for_switches(ui, query.width);
        asked.extend(top::query_row(state, ui, query));
        if !has_room {
            asked.extend(top::text_switches_row(state, ui, area.top(control)));
        }
        if let Some(refused) = state.refused {
            let line = area.top(theme.sizes.row_height);
            let _ = ui.label(line, &Label::new(refused.trim()).tone(Tone::Error));
        }
        asked.extend(top::choices_row(state, ui, area.top(control)));
        asked.extend(top::filters_row(state, ui, area.top(control)));
        results::count_line(state, ui, area.bottom(theme.sizes.row_height));
        asked.extend(results::results_view(state, ui, area.rest()));
        asked.extend(keys(state, ui));
    });
    asked.into_iter().map(Intent::FileSearch).collect()
}

/// The modifier keys held with the keys of the page: `Alt`, and `Ctrl` too on macOS, where `Option` with a key types
/// a character.
pub const KEYS_HELD: Modifiers = if cfg!(target_os = "macos") {
    Modifiers::ALT.with(Modifiers::CONTROL)
} else {
    Modifiers::ALT
};

/// The keys of the page that no widget used: `Alt+1` to `Alt+4` choose a mode, `Alt+C`, `Alt+W` and `Alt+R` turn the
/// switches of the text; with `Ctrl` too on macOS, where `Option` with a key types a character.
fn keys(state: &SearchPageState<'_>, ui: &Ui<'_>) -> Vec<FileSearchIntent> {
    ui.unclaimed_keys()
        .into_iter()
        .filter(|key| key.modifiers == KEYS_HELD)
        .filter_map(|key| {
            let mode = match key.key {
                Key::Character('1') => Mode::All,
                Key::Character('2') => Mode::Text,
                Key::Character('3') => Mode::Files,
                Key::Character('4') => Mode::Folders,
                Key::Character('c' | 'C') => return Some(FileSearchIntent::Turn(Switch::MatchCase)),
                Key::Character('w' | 'W') => return Some(FileSearchIntent::Turn(Switch::WholeWord)),
                Key::Character('r' | 'R') => return Some(FileSearchIntent::Turn(Switch::Regex)),
                _ => return None,
            };
            (mode != state.search.mode).then_some(FileSearchIntent::Mode(mode))
        })
        .collect()
}
