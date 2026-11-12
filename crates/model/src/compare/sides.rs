//! The two sides of a comparison as its page shows them: followed as their names move, or fixed at a commit, and how
//! they are compared at first.

use super::{CompareMode, Side};
use crate::CommitId;

/// The name of the working tree as a side.
pub const WORKING_TREE: &str = "Working tree";

/// A side of a comparison, as the page shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompareSide {
    /// Its name: a branch, a tag, a revision, a hash, or **Working tree**.
    pub name: String,
    /// Where it is now.
    pub side: Side,
    /// Whether it follows its name as it moves; a side given by its hash is fixed.
    pub is_followed: bool,
    /// Whether its name no longer stands for a commit: it stays at its last commit.
    pub is_gone: bool,
}

impl CompareSide {
    /// The working tree as a side.
    #[must_use]
    pub fn working_tree() -> CompareSide {
        CompareSide {
            name: String::from(WORKING_TREE),
            side: Side::WorkingTree,
            is_followed: true,
            is_gone: false,
        }
    }

    /// Whether it is the working tree.
    #[must_use]
    pub fn is_working_tree(&self) -> bool {
        self.side == Side::WorkingTree
    }

    /// What its tooltip says it is.
    #[must_use]
    pub fn tooltip(&self) -> String {
        let what = if self.is_working_tree() {
            String::from("The files as they are now, with the changes not committed")
        } else if self.is_gone {
            format!(
                "`{}` no longer stands for a commit: the comparison stays where it was",
                self.name
            )
        } else if self.is_followed {
            format!("The comparison follows `{}` as it moves", self.name)
        } else {
            String::from("This commit")
        };
        format!("{what}. Choose another branch, tag or commit")
    }
}

/// What [`follow_sides`] looks at: a side as chosen, and what its name resolves to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideInput<'a> {
    /// The name it was chosen by.
    pub name: &'a str,
    /// Where it was when it was chosen, or last shown.
    pub chosen: &'a Side,
    /// Whether it follows its name; none when it is being chosen, and this decides it.
    pub is_followed: Option<bool>,
    /// What git resolves its name to now; none when the name no longer stands for a commit.
    pub resolved: Option<&'a CommitId>,
}

/// A side as the page shows it.
///
/// A side is followed when its name is not the start of its own hash and git resolves the name to that very commit when
/// it is chosen; the working tree is always followed. A followed side moves to where its name is now, or stays at its
/// last commit, marked gone, when the name no longer resolves; a fixed side stays its commit.
///
/// Feature: compare.follow-sides
#[must_use]
pub fn follow_sides(input: &SideInput<'_>) -> CompareSide {
    let Side::Commit(commit) = input.chosen else {
        return CompareSide::working_tree();
    };
    let is_followed = input.is_followed.unwrap_or_else(|| {
        let is_own_hash = !input.name.is_empty() && commit.as_str().starts_with(&input.name.to_lowercase());
        !is_own_hash && input.resolved == Some(commit)
    });
    let (side, is_gone) = match (is_followed, input.resolved) {
        (true, Some(now)) => (Side::Commit(now.clone()), false),
        (true, None) => (Side::Commit(commit.clone()), true),
        (false, _) => (Side::Commit(commit.clone()), false),
    };
    CompareSide {
        name: input.name.to_owned(),
        side,
        is_followed,
        is_gone,
    }
}

/// Whether two comparisons are the same page: the same names, both followed or both fixed and then at the same commits,
/// in the same order, narrowed to the same path. The repository is the caller's to compare.
#[must_use]
pub fn is_same_page(
    (first, second, path): (&CompareSide, &CompareSide, Option<&str>),
    (other_first, other_second, other_path): (&CompareSide, &CompareSide, Option<&str>),
) -> bool {
    let same = |side: &CompareSide, other: &CompareSide| {
        side.name == other.name
            && side.is_followed == other.is_followed
            && (side.is_followed || side.side == other.side)
    };
    same(first, other_first) && same(second, other_second) && path == other_path
}

/// How two sides are compared at first: **Since they diverged** when both are followed and neither is the working
/// tree, **Everything that differs** otherwise.
///
/// Feature: compare.initial-mode
#[must_use]
pub fn initial_mode(input: &(CompareSide, CompareSide)) -> CompareMode {
    let (first, second) = input;
    let is_named = |side: &CompareSide| side.is_followed && !side.is_working_tree();
    if is_named(first) && is_named(second) {
        CompareMode::SinceDiverged
    } else {
        CompareMode::Everything
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{CompareSide, SideInput, follow_sides, initial_mode, is_same_page};
    use crate::CommitId;
    use crate::compare::{CompareMode, Side};

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
    }

    /// A side chosen by a name that resolves to its commit.
    fn chosen(name: &str, commit: &CommitId) -> CompareSide {
        let side = Side::Commit(commit.clone());
        follow_sides(&SideInput {
            name,
            chosen: &side,
            is_followed: None,
            resolved: Some(commit),
        })
    }

    /// A side shown again, its name resolving to `now`.
    fn again(side: &CompareSide, now: Option<&CommitId>) -> CompareSide {
        follow_sides(&SideInput {
            name: &side.name,
            chosen: &side.side,
            is_followed: Some(side.is_followed),
            resolved: now,
        })
    }

    #[test]
    fn compare_follow_sides__a_branch_moves_to_where_it_is_now() {
        let feature = chosen("feature", &id('1'));
        assert!(feature.is_followed);
        let moved = again(&feature, Some(&id('2')));
        assert_eq!(moved.side, Side::Commit(id('2')));
        assert!(!moved.is_gone);
        assert_eq!(
            moved.tooltip(),
            "The comparison follows `feature` as it moves. Choose another branch, tag or commit"
        );
    }

    #[test]
    fn compare_follow_sides__a_commit_given_by_its_hash_stays_after_its_branch_moves() {
        let commit = chosen("1111111", &id('1'));
        assert!(!commit.is_followed);
        assert_eq!(again(&commit, Some(&id('2'))).side, Side::Commit(id('1')));
        assert_eq!(commit.tooltip(), "This commit. Choose another branch, tag or commit");
        let other = Side::Commit(id('1'));
        let tag_named_like_a_branch = follow_sides(&SideInput {
            name: "main",
            chosen: &other,
            is_followed: None,
            resolved: Some(&id('3')),
        });
        assert!(
            !tag_named_like_a_branch.is_followed,
            "a name git reads as another commit stays the commit chosen"
        );
        assert_eq!(tag_named_like_a_branch.side, Side::Commit(id('1')));
    }

    #[test]
    fn compare_follow_sides__a_branch_deleted_stays_at_its_last_commit_marked_gone() {
        let feature = chosen("feature", &id('1'));
        let gone = again(&feature, None);
        assert_eq!((gone.side.clone(), gone.is_gone), (Side::Commit(id('1')), true));
        assert_eq!(
            gone.tooltip(),
            "`feature` no longer stands for a commit: the comparison stays where it was. Choose another branch, tag or \
             commit"
        );
    }

    #[test]
    fn compare_follow_sides__the_same_names_in_the_same_order_are_one_page() {
        let (main, feature) = (chosen("main", &id('1')), chosen("feature", &id('2')));
        assert!(is_same_page((&main, &feature, None), (&main, &feature, None)));
        assert!(!is_same_page((&main, &feature, None), (&feature, &main, None)));
        assert!(!is_same_page((&main, &feature, None), (&main, &feature, Some("src"))));
        let moved = again(&feature, Some(&id('3')));
        assert!(
            is_same_page((&main, &feature, None), (&main, &moved, None)),
            "followed sides are their names"
        );
    }

    #[test]
    fn compare_initial_mode__two_branches_open_since_they_diverged() {
        let sides = (chosen("main", &id('1')), chosen("feature", &id('2')));
        assert_eq!(initial_mode(&sides), CompareMode::SinceDiverged);
    }

    #[test]
    fn compare_initial_mode__a_commit_given_by_its_hash_opens_with_everything() {
        let sides = (chosen("main", &id('1')), chosen("2222222", &id('2')));
        assert_eq!(initial_mode(&sides), CompareMode::Everything);
    }

    #[test]
    fn compare_initial_mode__the_working_tree_opens_with_everything() {
        let sides = (chosen("main", &id('1')), CompareSide::working_tree());
        assert_eq!(initial_mode(&sides), CompareMode::Everything);
        assert_eq!(
            CompareSide::working_tree().tooltip(),
            "The files as they are now, with the changes not committed. Choose another branch, tag or commit"
        );
    }
}
