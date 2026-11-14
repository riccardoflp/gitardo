//! The entries of the menu of a branch, a remote branch or a tag that change the branches the timeline shows: **Show
//! only this history**, **Show this history too**, **Hide this history** and **Show all branches**, each offered when
//! its card says.

use model::filter::{BranchesShown, hide, show_all, show_only, show_too, ticked};
use model::refs::References;

use crate::intent::{FilterIntent, Intent};
use crate::text::filter as words;

/// An entry of the menu of a reference about the branches shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FilterEntry {
    /// **Show only this history** (`filter.show-only`).
    ShowOnly,
    /// **Show this history too** (`filter.show-too`).
    ShowToo,
    /// **Hide this history** (`filter.hide`).
    Hide,
    /// **Show all branches** (`filter.show-all`).
    ShowAll,
}

/// The entries offered on a reference, by its full name: on the reference already shown alone, **Show all branches**
/// alone; otherwise **Show only this history**, then while the timeline is narrowed **Show this history too** when it
/// is not shown, **Hide this history** when it is and is not the only one, and **Show all branches**.
#[must_use]
pub fn filter_entries(shown: Option<&BranchesShown>, reference: &str, references: &References) -> Vec<FilterEntry> {
    let Some(narrowed) = shown else {
        return vec![FilterEntry::ShowOnly];
    };
    let names = ticked(Some(narrowed), references);
    let is_shown = names.iter().any(|name| name == reference);
    if is_shown && names.len() == 1 {
        return vec![FilterEntry::ShowAll];
    }
    let mut entries = vec![FilterEntry::ShowOnly];
    entries.push(if is_shown {
        FilterEntry::Hide
    } else {
        FilterEntry::ShowToo
    });
    entries.push(FilterEntry::ShowAll);
    entries
}

/// The words of an entry.
#[must_use]
pub fn filter_entry_words(entry: FilterEntry) -> &'static str {
    match entry {
        FilterEntry::ShowOnly => words::SHOW_ONLY,
        FilterEntry::ShowToo => words::SHOW_TOO,
        FilterEntry::Hide => words::HIDE,
        FilterEntry::ShowAll => words::SHOW_ALL_BRANCHES,
    }
}

/// What an entry chosen on a reference asks: the branches to show after it.
#[must_use]
pub fn filter_entry_intent(
    entry: FilterEntry,
    shown: Option<&BranchesShown>,
    reference: &str,
    references: &References,
) -> Intent {
    let after = match (entry, shown) {
        (FilterEntry::ShowOnly, _) => Some(show_only(reference)),
        (FilterEntry::ShowToo, Some(narrowed)) => Some(show_too(narrowed, reference, references)),
        (FilterEntry::Hide, Some(narrowed)) => Some(hide(narrowed, reference, references)),
        (FilterEntry::ShowAll | FilterEntry::ShowToo | FilterEntry::Hide, _) => show_all(),
    };
    Intent::Filter(FilterIntent::Show(after))
}
