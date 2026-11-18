//! What the Search page looks for and where, when it is asked for from a place of the window.

use super::{Place, Scope, Search};

/// How the Search page was asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Asked {
    /// From the bar of views, the key or Quick search, with the text selected in the page in front, if any.
    Page {
        /// The text selected in the File page or the Diff page in front.
        selected: Option<String>,
    },
    /// **Search here**, on a folder or a file of the tree of the Files page; none for its top row.
    Here(Option<Place>),
}

/// The search as the page shows it once asked for: the text selected, when it is on one line, becomes what is looked
/// for, and **Search here** makes the folder or the file it names the scope, in place of the one it named before.
/// Everything else stays as it was left.
///
/// Feature: file-search.open
#[must_use]
pub fn open_with(search: &Search, asked: &Asked) -> Search {
    let mut search = search.clone();
    match asked {
        Asked::Page { selected } => {
            let one_line = selected
                .as_deref()
                .filter(|text| !text.trim().is_empty() && !text.contains(['\n', '\r']));
            if let Some(text) = one_line {
                text.clone_into(&mut search.query);
            }
        }
        Asked::Here(Some(place)) => {
            search.named = Some(place.clone());
            search.scope = Scope::Named;
        }
        Asked::Here(None) => search.scope = Scope::Repository,
    }
    search
}
