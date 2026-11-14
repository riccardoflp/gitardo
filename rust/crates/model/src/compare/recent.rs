//! The comparisons opened before in a repository, so that one can be opened again as it was.

use super::Side;
use super::sides::WORKING_TREE;
use crate::CommitId;

/// The most comparisons remembered for a repository.
pub const RECENT_LIMIT: usize = 8;

/// A side of a comparison opened before: its name, and the commit it was at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentSide {
    /// Its name: a branch, a tag, a revision, a hash, or **Working tree**.
    pub name: String,
    /// The commit it was at; none for the working tree.
    pub commit: Option<CommitId>,
}

/// A comparison opened before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentComparison {
    /// The first side.
    pub first: RecentSide,
    /// The second side.
    pub second: RecentSide,
    /// The path it was narrowed to.
    pub path: Option<String>,
}

impl RecentComparison {
    /// Whether it is the same comparison as another, either way round.
    fn is_same(&self, other: &RecentComparison) -> bool {
        let names = (&self.first.name, &self.second.name);
        let same_way = names == (&other.first.name, &other.second.name);
        let other_way = names == (&other.second.name, &other.first.name);
        (same_way || other_way) && self.path == other.path
    }
}

/// What [`recent`] looks at.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecentInput<'a> {
    /// The comparisons remembered for the repository, the latest first.
    pub remembered: &'a [RecentComparison],
    /// The comparison just opened, swapped or narrowed.
    pub opened: &'a RecentComparison,
}

/// The comparisons remembered once one is opened: it first, without the same comparison either way round, at most
/// [`RECENT_LIMIT`].
///
/// Feature: compare.recent
#[must_use]
pub fn recent(input: &RecentInput<'_>) -> Vec<RecentComparison> {
    std::iter::once(input.opened.clone())
        .chain(
            input
                .remembered
                .iter()
                .filter(|known| !known.is_same(input.opened))
                .cloned(),
        )
        .take(RECENT_LIMIT)
        .collect()
}

/// Where a side of a comparison opened before is when it is opened again: the working tree stays the working tree, a
/// side named by its hash, or the start of it, stays that commit, and any other name is where it resolves now, or at
/// the commit remembered when it no longer resolves.
#[must_use]
pub fn reopened(side: &RecentSide, resolved: Option<&CommitId>) -> Side {
    let Some(commit) = &side.commit else {
        return Side::WorkingTree;
    };
    if side.name == WORKING_TREE {
        return Side::WorkingTree;
    }
    let is_own_hash = !side.name.is_empty() && commit.as_str().starts_with(&side.name.to_lowercase());
    match resolved {
        Some(now) if !is_own_hash => Side::Commit(now.clone()),
        _ => Side::Commit(commit.clone()),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{RecentComparison, RecentInput, RecentSide, recent, reopened};
    use crate::CommitId;
    use crate::compare::Side;

    fn id(digit: char) -> CommitId {
        CommitId::parse(&digit.to_string().repeat(40)).unwrap_or_else(|| panic!("a hash"))
    }

    fn side(name: &str) -> RecentSide {
        RecentSide {
            name: name.to_owned(),
            commit: Some(id('1')),
        }
    }

    fn comparison(first: &str, second: &str, path: Option<&str>) -> RecentComparison {
        RecentComparison {
            first: side(first),
            second: side(second),
            path: path.map(str::to_owned),
        }
    }

    fn names(list: &[RecentComparison]) -> Vec<String> {
        list.iter()
            .map(|known| format!("{} {}", known.first.name, known.second.name))
            .collect()
    }

    #[test]
    fn compare_recent__a_ninth_comparison_drops_the_oldest_of_eight() {
        let remembered: Vec<RecentComparison> = (1..=8)
            .map(|index| comparison("main", &format!("topic-{index}"), None))
            .collect();
        let opened = comparison("main", "topic-9", None);
        let list = recent(&RecentInput {
            remembered: &remembered,
            opened: &opened,
        });
        assert_eq!(list.len(), 8);
        assert_eq!(names(&list).first().map(String::as_str), Some("main topic-9"));
        assert!(!names(&list).contains(&String::from("main topic-8")));
    }

    #[test]
    fn compare_recent__the_same_pair_the_other_way_round_is_one_entry_the_latest_first() {
        let remembered = [comparison("feature", "main", None), comparison("main", "other", None)];
        let opened = comparison("main", "feature", None);
        let list = recent(&RecentInput {
            remembered: &remembered,
            opened: &opened,
        });
        assert_eq!(names(&list), ["main feature", "main other"]);
    }

    #[test]
    fn compare_recent__narrowed_and_not_narrowed_are_two_entries() {
        let remembered = [comparison("main", "feature", None)];
        let opened = comparison("main", "feature", Some("src"));
        let list = recent(&RecentInput {
            remembered: &remembered,
            opened: &opened,
        });
        assert_eq!(list.len(), 2);
    }

    #[test]
    fn compare_recent__a_branch_opened_again_is_where_it_is_now() {
        assert_eq!(reopened(&side("feature"), Some(&id('2'))), Side::Commit(id('2')));
        assert_eq!(
            reopened(&side("feature"), None),
            Side::Commit(id('1')),
            "no longer resolves"
        );
        assert_eq!(
            reopened(&side("1111111"), Some(&id('2'))),
            Side::Commit(id('1')),
            "a hash stays"
        );
        let working_tree = RecentSide {
            name: String::from("Working tree"),
            commit: None,
        };
        assert_eq!(reopened(&working_tree, Some(&id('2'))), Side::WorkingTree);
    }
}
