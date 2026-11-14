//! What the branches shown are called, on the button of the branch filter and on the line at the top of the branch
//! rail. The card gives the words with the derivation, as `graph.date-separators` does (Q10).

use super::{BranchesShown, Kinds, short_name};

/// What the branches shown are called: the words, and for references ticked one by one, how many more there are
/// after the first, which the button of the filter draws apart so that the count stays when the name is cut.
#[must_use]
pub fn label_parts(shown: Option<&BranchesShown>) -> (String, Option<usize>) {
    match shown {
        None => (String::from("All branches"), None),
        Some(BranchesShown::Current) => (String::from("Current branch"), None),
        Some(BranchesShown::Upstream) => (String::from("Current and upstream"), None),
        Some(BranchesShown::Kinds(kinds)) => (String::from(kinds_words(*kinds)), None),
        Some(BranchesShown::References(names)) => {
            let first = names.first().map_or_default(|name| short_name(name));
            let more = names.len().saturating_sub(1);
            (first.to_owned(), (more > 0).then_some(more))
        }
    }
}

/// What the branches shown are called: `All branches`, `Current branch`, `Local branches and tags`, `main`, `main +2`.
///
/// Feature: filter.label
#[must_use]
pub fn label(shown: Option<&BranchesShown>) -> String {
    match label_parts(shown) {
        (words, Some(more)) => format!("{words} +{more}"),
        (words, None) => words,
    }
}

/// The words of some kinds of references.
fn kinds_words(kinds: Kinds) -> &'static str {
    match (kinds.local, kinds.remote, kinds.tags) {
        (true, true, true) | (false, false, false) => "All branches",
        (true, false, false) => "Local branches",
        (false, true, false) => "Remote branches",
        (false, false, true) => "Tags",
        (true, true, false) => "Branches",
        (true, false, true) => "Local branches and tags",
        (false, true, true) => "Remote branches and tags",
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::super::{BranchesShown, Kinds, ReferenceKind};
    use super::{label, label_parts};

    fn kinds(local: bool, remote: bool, tags: bool) -> Option<BranchesShown> {
        (local || remote || tags).then_some(BranchesShown::Kinds(Kinds { local, remote, tags }))
    }

    fn ticked(names: &[&str]) -> Option<BranchesShown> {
        (!names.is_empty()).then(|| BranchesShown::References(names.iter().map(|name| (*name).to_owned()).collect()))
    }

    #[test]
    fn filter_label__each_row_of_the_table_gives_its_label() {
        let table = [
            (None, "All branches"),
            (Some(BranchesShown::Current), "Current branch"),
            (Some(BranchesShown::Upstream), "Current and upstream"),
            (kinds(true, false, false), "Local branches"),
            (kinds(false, true, false), "Remote branches"),
            (kinds(false, false, true), "Tags"),
            (kinds(true, true, false), "Branches"),
            (kinds(true, false, true), "Local branches and tags"),
            (kinds(false, true, true), "Remote branches and tags"),
            (ticked(&["refs/heads/main"]), "main"),
            (
                ticked(&["refs/heads/main", "refs/heads/feature", "refs/tags/v1.0"]),
                "main +2",
            ),
        ];
        for (shown, words) in table {
            assert_eq!(label(shown.as_ref()), words, "{shown:?}");
        }
    }

    #[test]
    fn filter_label__a_remote_branch_ticked_alone_is_labeled_by_its_short_name() {
        assert_eq!(label(ticked(&["refs/remotes/origin/main"]).as_ref()), "origin/main");
        let full = ReferenceKind::Remote.full_name("origin/main");
        assert_eq!(full, "refs/remotes/origin/main");
    }

    #[test]
    fn filter_label__the_count_is_apart_from_the_first_name() {
        let shown = ticked(&["refs/heads/feature", "refs/heads/fix"]);
        assert_eq!(label_parts(shown.as_ref()), (String::from("feature"), Some(1)));
    }
}
