//! The sentences under the sides of a comparison: how far apart they are, where they diverged, and what merging would
//! do.

use super::sides::CompareSide;
use super::{CompareView, MergeOutlook, Side};
use crate::CommitId;

/// How many files in conflict the sentence of merging names before it counts the others.
const NAMED_FILES: usize = 10;

/// What [`summary`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SummaryInput<'a> {
    /// The first side.
    pub first: &'a CompareSide,
    /// The second side.
    pub second: &'a CompareSide,
    /// How far apart they are, where they diverged, and what merging would do.
    pub view: &'a CompareView,
}

/// Where the two sides diverged.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Diverged {
    /// At a commit, which the sentence opens.
    At {
        /// The sentence.
        text: String,
        /// The commit.
        commit: CommitId,
    },
    /// **Their histories never meet.**
    NeverMeet,
}

/// What merging the second side into the first would do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Merging {
    /// The sentence.
    pub text: String,
    /// The files it would conflict in, the first 10.
    pub files: Vec<String>,
    /// How many more files it would conflict in.
    pub more: usize,
}

/// The sentences under the sides.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareSummary {
    /// How far apart they are.
    pub distance: String,
    /// Where they diverged, when it is said.
    pub diverged: Option<Diverged>,
    /// What merging would do, when it is said.
    pub merging: Option<Merging>,
}

/// `1 commit`, `3 commits`.
fn commits(count: usize) -> String {
    if count == 1 {
        String::from("1 commit")
    } else {
        format!("{count} commits")
    }
}

/// The sentences under the sides of a comparison.
///
/// Feature: compare.summary
#[must_use]
pub fn summary(input: &SummaryInput<'_>) -> CompareSummary {
    let view = input.view;
    let has_working_tree = input.first.is_working_tree() || input.second.is_working_tree();
    let distance = if has_working_tree {
        let (other, ahead, behind) = if input.first.is_working_tree() {
            (input.second, view.only_in_first, view.only_in_second)
        } else {
            (input.first, view.only_in_second, view.only_in_first)
        };
        if ahead == 0 && behind == 0 {
            format!(
                "The commit checked out is `{}`: only the changes not committed differ.",
                other.name
            )
        } else {
            distance("The commit checked out", &other.name, ahead, behind)
        }
    } else if view.only_in_first == 0 && view.only_in_second == 0 {
        String::from("The two are the same commit.")
    } else {
        distance(
            &format!("`{}`", input.second.name),
            &input.first.name,
            view.only_in_second,
            view.only_in_first,
        )
    };
    CompareSummary {
        distance,
        diverged: diverged(input, has_working_tree),
        merging: if has_working_tree { None } else { merging(input) },
    }
}

/// The sentence of distance of a subject against a side.
fn distance(subject: &str, other: &str, ahead: usize, behind: usize) -> String {
    match (ahead, behind) {
        (0, behind) => format!(
            "{subject} is {} behind `{other}`, with none of its own.",
            commits(behind)
        ),
        (ahead, 0) => format!("{subject} is {} ahead of `{other}`.", commits(ahead)),
        (ahead, behind) => format!(
            "{subject} is {} ahead of `{other}` and {} behind.",
            commits(ahead),
            commits(behind)
        ),
    }
}

/// Where the sides diverged: at their merge base when it is neither side, or nowhere.
fn diverged(input: &SummaryInput<'_>, has_working_tree: bool) -> Option<Diverged> {
    let Some(base) = &input.view.merge_base else {
        return Some(Diverged::NeverMeet);
    };
    let is_a_side = [input.first, input.second]
        .iter()
        .any(|side| side.side == Side::Commit(base.clone()));
    (!has_working_tree && !is_a_side).then(|| Diverged::At {
        text: format!("They diverged at `{}`.", base.short()),
        commit: base.clone(),
    })
}

/// What merging the second side into the first would do, when the second has commits of its own.
fn merging(input: &SummaryInput<'_>) -> Option<Merging> {
    let view = input.view;
    if view.only_in_second == 0 {
        return None;
    }
    let start = format!("Merging `{}` into `{}` would", input.second.name, input.first.name);
    let plain = |end: &str| Merging {
        text: format!("{start} {end}"),
        files: Vec::new(),
        more: 0,
    };
    if view.only_in_first == 0 {
        return Some(plain("fast-forward it."));
    }
    match &view.merge {
        MergeOutlook::Clean => Some(plain("not conflict.")),
        MergeOutlook::Conflicts(files) => {
            let count = if files.len() == 1 {
                String::from("1 file")
            } else {
                format!("{} files", files.len())
            };
            Some(Merging {
                text: format!("{start} conflict in {count}:"),
                files: files.iter().take(NAMED_FILES).cloned().collect(),
                more: files.len().saturating_sub(NAMED_FILES),
            })
        }
        MergeOutlook::NotAsked | MergeOutlook::Unknown => None,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{Diverged, SummaryInput, summary};
    use crate::CommitId;
    use crate::compare::{CompareSide, CompareView, MergeOutlook, Side};

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
    }

    fn named(name: &str, commit: &CommitId) -> CompareSide {
        CompareSide {
            name: name.to_owned(),
            side: Side::Commit(commit.clone()),
            is_followed: true,
            is_gone: false,
        }
    }

    fn view(base: Option<CommitId>, only_in_first: usize, only_in_second: usize, merge: MergeOutlook) -> CompareView {
        CompareView {
            merge_base: base,
            only_in_first,
            only_in_second,
            files: Vec::new(),
            commits: Vec::new(),
            merge,
        }
    }

    #[test]
    fn compare_summary__a_branch_ahead_with_none_behind_would_fast_forward() {
        let (main, feature) = (named("main", &id('1')), named("feature", &id('2')));
        let view = view(Some(id('1')), 0, 3, MergeOutlook::NotAsked);
        let said = summary(&SummaryInput {
            first: &main,
            second: &feature,
            view: &view,
        });
        assert_eq!(said.distance, "`feature` is 3 commits ahead of `main`.");
        assert_eq!(said.diverged, None, "the merge base is a side");
        assert_eq!(
            said.merging.map(|merging| merging.text).as_deref(),
            Some("Merging `feature` into `main` would fast-forward it.")
        );
    }

    #[test]
    fn compare_summary__a_conflict_in_12_files_names_10_and_2_more() {
        let (main, feature) = (named("main", &id('1')), named("feature", &id('2')));
        let files: Vec<String> = (1..=12).map(|index| format!("src/file-{index}.rs")).collect();
        let view = view(Some(id('3')), 1, 2, MergeOutlook::Conflicts(files));
        let said = summary(&SummaryInput {
            first: &main,
            second: &feature,
            view: &view,
        });
        assert_eq!(
            said.distance,
            "`feature` is 2 commits ahead of `main` and 1 commit behind."
        );
        assert_eq!(
            said.diverged,
            Some(Diverged::At {
                text: String::from("They diverged at `3333333`."),
                commit: id('3'),
            })
        );
        let merging = said.merging.unwrap_or_else(|| panic!("merging is said"));
        assert_eq!(
            merging.text,
            "Merging `feature` into `main` would conflict in 12 files:"
        );
        assert_eq!((merging.files.len(), merging.more), (10, 2));
    }

    #[test]
    fn compare_summary__the_working_tree_at_head_says_only_the_changes_differ() {
        let main = named("main", &id('1'));
        let view = view(Some(id('1')), 0, 0, MergeOutlook::NotAsked);
        let said = summary(&SummaryInput {
            first: &main,
            second: &CompareSide::working_tree(),
            view: &view,
        });
        assert_eq!(
            said.distance,
            "The commit checked out is `main`: only the changes not committed differ."
        );
        assert_eq!((said.diverged, said.merging), (None, None));
        let behind = super::super::CompareView {
            only_in_first: 2,
            ..view
        };
        let said = summary(&SummaryInput {
            first: &main,
            second: &CompareSide::working_tree(),
            view: &behind,
        });
        assert_eq!(
            said.distance,
            "The commit checked out is 2 commits behind `main`, with none of its own."
        );
    }

    #[test]
    fn compare_summary__unrelated_histories_never_meet() {
        let (main, other) = (named("main", &id('1')), named("pages", &id('2')));
        let view = view(None, 4, 1, MergeOutlook::Unknown);
        let said = summary(&SummaryInput {
            first: &main,
            second: &other,
            view: &view,
        });
        assert_eq!(said.diverged, Some(Diverged::NeverMeet));
        assert_eq!(said.merging, None);
        let same = super::super::CompareView {
            merge_base: Some(id('1')),
            only_in_first: 0,
            only_in_second: 0,
            ..view
        };
        let said = summary(&SummaryInput {
            first: &main,
            second: &main,
            view: &same,
        });
        assert_eq!(said.distance, "The two are the same commit.");
    }
}
