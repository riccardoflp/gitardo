//! The branches shown, as kept for each repository in Gitardo's storage, and checked against the repository at each
//! read.

use std::collections::HashSet;

use super::{BranchesShown, Kinds, ReferenceKind, full_names};
use crate::refs::References;

/// How many references ticked one by one are kept: each is an argument of `git log`, and a command line has an end.
pub const MOST_TICKED: usize = 200;

/// The branches shown as they are kept, in words this version may or may not know.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StoredValue {
    /// A preset by its word: `current` or `upstream`.
    Preset(String),
    /// Some kinds by their words: `local`, `remote`, `tags`.
    Kinds(Vec<String>),
    /// References ticked one by one, by their full names.
    References(Vec<String>),
    /// A shape this version does not know.
    Unknown,
}

/// The word of a preset kept.
const CURRENT: &str = "current";
/// The word of the **Upstream** preset kept.
const UPSTREAM: &str = "upstream";

/// The word of a kind kept.
fn kind_word(kind: ReferenceKind) -> &'static str {
    match kind {
        ReferenceKind::Local => "local",
        ReferenceKind::Remote => "remote",
        ReferenceKind::Tag => "tags",
    }
}

impl StoredValue {
    /// How a choice is kept: at most [`MOST_TICKED`] references ticked one by one.
    #[must_use]
    pub fn of(shown: &BranchesShown) -> StoredValue {
        match shown {
            BranchesShown::Current => StoredValue::Preset(String::from(CURRENT)),
            BranchesShown::Upstream => StoredValue::Preset(String::from(UPSTREAM)),
            BranchesShown::Kinds(kinds) => StoredValue::Kinds(
                ReferenceKind::ALL
                    .into_iter()
                    .filter(|kind| kinds.has(*kind))
                    .map(|kind| String::from(kind_word(kind)))
                    .collect(),
            ),
            BranchesShown::References(names) => {
                StoredValue::References(names.iter().take(MOST_TICKED).cloned().collect())
            }
        }
    }
}

/// The branches shown, from what is kept for a repository and the references it has now: references ticked that no
/// longer exist are left out, and with none left everything is shown; what is kept in a shape this version does not
/// know shows everything.
///
/// Feature: filter.remember
#[must_use]
pub fn remember(stored: &StoredValue, references: &References) -> Option<BranchesShown> {
    match stored {
        StoredValue::Preset(word) if word == CURRENT => Some(BranchesShown::Current),
        StoredValue::Preset(word) if word == UPSTREAM => Some(BranchesShown::Upstream),
        StoredValue::Preset(_) | StoredValue::Unknown => None,
        StoredValue::Kinds(words) => kinds_of(words),
        StoredValue::References(names) => {
            let existing: HashSet<String> = full_names(references, Kinds::ALL).into_iter().collect();
            let kept: Vec<String> = names
                .iter()
                .take(MOST_TICKED)
                .filter(|name| existing.contains(*name))
                .cloned()
                .collect();
            (!kept.is_empty()).then_some(BranchesShown::References(kept))
        }
    }
}

/// A choice checked against the references a repository has now, as at each read: [`remember`] of how it is kept.
#[must_use]
pub fn checked(shown: Option<&BranchesShown>, references: &References) -> Option<BranchesShown> {
    shown.and_then(|shown| remember(&StoredValue::of(shown), references))
}

/// The kinds of their words: none, which is everything, for all three, for none, or for a word this version does not
/// know.
fn kinds_of(words: &[String]) -> Option<BranchesShown> {
    let mut kinds = Kinds::default();
    for word in words {
        let kind = ReferenceKind::ALL
            .into_iter()
            .find(|kind| kind_word(*kind) == word.as_str())?;
        kinds = kinds.with(kind, true);
    }
    (kinds.count() != 0 && kinds != Kinds::ALL).then_some(BranchesShown::Kinds(kinds))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::super::testing::references;
    use super::super::{BranchesShown, Kinds, ReferenceKind};
    use super::{MOST_TICKED, StoredValue, checked, remember};

    fn ticked(names: &[&str]) -> BranchesShown {
        BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect())
    }

    #[test]
    fn filter_remember__a_choice_kept_comes_back_as_it_was() {
        let repository = references(&["main", "feature"], &["origin/main"], &["v1.0"]);
        for shown in [
            BranchesShown::Current,
            BranchesShown::Upstream,
            BranchesShown::Kinds(Kinds::only(ReferenceKind::Local)),
            ticked(&["refs/heads/feature"]),
        ] {
            assert_eq!(remember(&StoredValue::of(&shown), &repository), Some(shown));
        }
    }

    #[test]
    fn filter_remember__a_deleted_branch_is_left_out_of_those_ticked() {
        let repository = references(&["main"], &[], &[]);
        let kept = StoredValue::of(&ticked(&["refs/heads/feature", "refs/heads/main"]));
        assert_eq!(remember(&kept, &repository), Some(ticked(&["refs/heads/main"])));
    }

    #[test]
    fn filter_remember__with_none_left_everything_is_shown() {
        let repository = references(&["main"], &[], &[]);
        assert_eq!(checked(Some(&ticked(&["refs/heads/feature"])), &repository), None);
    }

    #[test]
    fn filter_remember__a_shape_this_version_does_not_know_shows_everything() {
        let repository = references(&["main"], &[], &[]);
        for kept in [
            StoredValue::Unknown,
            StoredValue::Preset(String::from("nearby")),
            StoredValue::Kinds(vec![String::from("local"), String::from("notes")]),
        ] {
            assert_eq!(remember(&kept, &repository), None, "{kept:?}");
        }
    }

    #[test]
    fn filter_remember__at_most_200_references_are_kept() {
        let names: Vec<String> = (0..250).map(|number| format!("refs/heads/b{number}")).collect();
        let StoredValue::References(kept) = StoredValue::of(&BranchesShown::References(names)) else {
            panic!("references are kept as references");
        };
        assert_eq!(kept.len(), MOST_TICKED);
    }
}
