//! The order of the branches, the remote branches and the tags: by name, or the one committed to last first.

use std::cmp::Reverse;

use crate::Timestamp;
use crate::refs::References;

/// The order the branches, the remote branches and the tags are listed in: `branches.sortOrder`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortOrder {
    /// By name, the order git lists them in.
    #[default]
    Name,
    /// The latest first: the date of the commit, or when an annotated tag was made; among the same date, by name.
    Date,
}

/// The references in an order, the same for all three lists. By name is the order git lists them in, the bytes of the
/// names compared; by date, the latest first, and by name among the same date.
///
/// Feature: refs.sort
#[must_use]
pub fn sort(references: &References, order: SortOrder) -> References {
    let mut sorted = references.clone();
    match order {
        SortOrder::Name => {
            sorted.branches.sort_by(|left, right| left.name.cmp(&right.name));
            sorted.remote_branches.sort_by(|left, right| left.name.cmp(&right.name));
            sorted.tags.sort_by(|left, right| left.name.cmp(&right.name));
        }
        SortOrder::Date => {
            sorted
                .branches
                .sort_by(|left, right| by_date((left.date, &left.name), (right.date, &right.name)));
            sorted
                .remote_branches
                .sort_by(|left, right| by_date((left.date, &left.name), (right.date, &right.name)));
            sorted
                .tags
                .sort_by(|left, right| by_date((left.date, &left.name), (right.date, &right.name)));
        }
    }
    sorted
}

/// The order in force: `branches.sortOrder` when it is set; while it is not, by date when git's own `branch.sort` sorts
/// by the date of the commits (`-committerdate`, or `committerdate`), and by name otherwise.
#[must_use]
pub fn order_in_force(setting: Option<SortOrder>, branch_sort: Option<&str>) -> SortOrder {
    match (setting, branch_sort.map(str::trim)) {
        (Some(order), _) => order,
        (None, Some("-committerdate" | "committerdate")) => SortOrder::Date,
        (None, _) => SortOrder::Name,
    }
}

/// The latest first, and by name among the same date.
fn by_date(left: (Timestamp, &String), right: (Timestamp, &String)) -> std::cmp::Ordering {
    (Reverse(left.0), left.1).cmp(&(Reverse(right.0), right.1))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{SortOrder, order_in_force, sort};
    use crate::Timestamp;
    use crate::graph::testing::{branch, id, remote_branch};
    use crate::refs::References;
    use crate::state::Tag;

    const NOW: i64 = 1_794_398_400;
    const HOUR: i64 = 3_600;

    fn dated_branch(name: &str, date: i64) -> crate::state::Branch {
        let mut branch = branch(name, "A", None);
        branch.date = Timestamp(date);
        branch
    }

    fn tag(name: &str, date: i64) -> Tag {
        Tag {
            name: name.to_owned(),
            commit: id("A"),
            date: Timestamp(date),
        }
    }

    fn branch_names(references: &References) -> Vec<&str> {
        references.branches.iter().map(|branch| branch.name.as_str()).collect()
    }

    #[test]
    fn refs_sort__by_date_a_branch_committed_to_an_hour_ago_comes_before_one_of_yesterday() {
        let references = References {
            branches: vec![dated_branch("alpha", NOW - 24 * HOUR), dated_branch("beta", NOW - HOUR)],
            ..References::default()
        };
        assert_eq!(branch_names(&sort(&references, SortOrder::Date)), ["beta", "alpha"]);
        assert_eq!(branch_names(&sort(&references, SortOrder::Name)), ["alpha", "beta"]);
    }

    #[test]
    fn refs_sort__an_annotated_tag_made_today_on_an_old_commit_comes_first_by_date() {
        // An annotated tag is dated when it was made, as `repo.read-state` reads it; a lightweight one by its commit.
        let references = References {
            tags: vec![
                tag("v1.0", NOW - 300 * 24 * HOUR),
                tag("v2.0", NOW - 2 * HOUR),
                tag("v0.9", NOW),
            ],
            ..References::default()
        };
        let sorted = sort(&references, SortOrder::Date);
        let names: Vec<&str> = sorted.tags.iter().map(|tag| tag.name.as_str()).collect();
        assert_eq!(names, ["v0.9", "v2.0", "v1.0"]);
    }

    #[test]
    fn refs_sort__one_order_sorts_the_branches_the_remote_branches_and_the_tags() {
        let mut old_remote = remote_branch("origin/a", "A");
        old_remote.date = Timestamp(NOW - HOUR);
        let mut new_remote = remote_branch("origin/b", "A");
        new_remote.date = Timestamp(NOW);
        let references = References {
            branches: vec![dated_branch("a", NOW - HOUR), dated_branch("b", NOW)],
            remote_branches: vec![old_remote, new_remote],
            tags: vec![tag("a", NOW - HOUR), tag("b", NOW)],
        };
        let sorted = sort(&references, SortOrder::Date);
        assert_eq!(branch_names(&sorted), ["b", "a"]);
        assert_eq!(sorted.remote_branches[0].name, "origin/b");
        assert_eq!(sorted.tags[0].name, "b");
    }

    #[test]
    fn refs_sort__not_set_it_is_by_name_unless_git_sorts_branches_by_the_date_of_their_commits() {
        assert_eq!(order_in_force(None, None), SortOrder::Name);
        assert_eq!(order_in_force(None, Some("refname")), SortOrder::Name);
        assert_eq!(order_in_force(None, Some("-committerdate")), SortOrder::Date);
        assert_eq!(order_in_force(None, Some("committerdate")), SortOrder::Date);
        assert_eq!(
            order_in_force(Some(SortOrder::Name), Some("-committerdate")),
            SortOrder::Name,
            "the setting set wins"
        );
        assert_eq!(order_in_force(Some(SortOrder::Date), None), SortOrder::Date);
    }

    #[test]
    fn refs_sort__the_same_date_is_ordered_by_name() {
        let references = References {
            branches: vec![
                dated_branch("zeta", NOW),
                dated_branch("alpha", NOW),
                dated_branch("Beta", NOW),
            ],
            ..References::default()
        };
        assert_eq!(
            branch_names(&sort(&references, SortOrder::Date)),
            ["Beta", "alpha", "zeta"]
        );
    }
}
