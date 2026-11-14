//! The stashes in the timeline: each above the commit it was made on, where its date puts it.

use crate::graph::{Row, StashCommit, place_stashes};

/// The rows of the timeline and the stashes to put among them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GraphWithStashes<'a> {
    /// The commits of the timeline, in their order.
    pub commits: &'a [Row],
    /// The stashes, with their commits.
    pub stashes: &'a [StashCommit],
    /// Whether **Show stashes** is checked (`graph.showStashes`).
    pub is_shown: bool,
    /// Whether the timeline shows what a search finds or the history of a file.
    pub is_narrowed: bool,
}

/// The rows of the timeline with the stashes among the commits: none while **Show stashes** is unchecked or the
/// timeline shows a search or the history of a file; otherwise each stash whose commit is loaded, above it, as
/// `graph.place-stashes` places it.
///
/// Feature: stash.in-graph
#[must_use]
pub fn in_graph(input: &GraphWithStashes<'_>) -> Vec<Row> {
    if !input.is_shown || input.is_narrowed {
        return input.commits.to_vec();
    }
    place_stashes(input.commits, input.stashes)
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{GraphWithStashes, in_graph};
    use crate::graph::testing::{commit_at, id};
    use crate::graph::{Row, StashCommit};

    const HOUR: i64 = 3_600;

    fn stash(name: &str, index: usize, base: &str, date: i64) -> StashCommit {
        StashCommit {
            commit: commit_at(name, &[base, &format!("{name}i")], date),
            index,
        }
    }

    fn timeline() -> Vec<Row> {
        vec![
            Row::Commit(commit_at("b", &["a"], 10 * HOUR)),
            Row::Commit(commit_at("a", &[], 5 * HOUR)),
        ]
    }

    fn names(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Stash { index, .. } => format!("stash@{{{index}}}"),
                other => other.id().map_or_default(|id| id.short().to_owned()),
            })
            .collect()
    }

    fn shown<'a>(commits: &'a [Row], stashes: &'a [StashCommit]) -> GraphWithStashes<'a> {
        GraphWithStashes {
            commits,
            stashes,
            is_shown: true,
            is_narrowed: false,
        }
    }

    #[test]
    fn stash_in_graph__a_stash_on_the_commit_checked_out_is_right_above_it_on_its_line() {
        let commits = timeline();
        let stashes = [stash("s", 0, "b", 11 * HOUR)];
        let rows = in_graph(&shown(&commits, &stashes));
        assert_eq!(names(&rows), ["stash@{0}", id("b").short(), id("a").short()]);
        let Some(Row::Stash { commit, .. }) = rows.first() else {
            panic!("the stash is first")
        };
        assert_eq!(commit.parents, [id("b")], "joined to its commit only");
    }

    #[test]
    fn stash_in_graph__a_stash_on_a_commit_not_shown_is_left_out() {
        let commits = timeline();
        let stashes = [stash("s", 0, "elsewhere", 11 * HOUR)];
        assert_eq!(in_graph(&shown(&commits, &stashes)), commits);
    }

    #[test]
    fn stash_in_graph__show_stashes_unchecked_takes_them_out_and_checked_brings_them_back() {
        let commits = timeline();
        let stashes = [stash("s", 0, "b", 11 * HOUR)];
        let mut input = shown(&commits, &stashes);
        input.is_shown = false;
        assert_eq!(in_graph(&input), commits);
        input.is_shown = true;
        assert_eq!(in_graph(&input).len(), 3);
    }

    #[test]
    fn stash_in_graph__no_stash_while_a_search_or_the_history_of_a_file_is_shown() {
        let commits = timeline();
        let stashes = [stash("s", 0, "b", 11 * HOUR)];
        let mut input = shown(&commits, &stashes);
        input.is_narrowed = true;
        assert_eq!(in_graph(&input), commits);
    }

    #[test]
    fn stash_in_graph__two_stashes_on_one_commit_are_newest_first_above_it() {
        let commits = timeline();
        let stashes = [stash("older", 1, "a", 6 * HOUR), stash("newer", 0, "a", 7 * HOUR)];
        let rows = in_graph(&shown(&commits, &stashes));
        assert_eq!(
            names(&rows),
            [id("b").short(), "stash@{0}", "stash@{1}", id("a").short()]
        );
    }
}
