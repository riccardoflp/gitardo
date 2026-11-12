//! Where the stashes go among the commits of the graph: each above the commit it was made on, where its date puts it.

use std::collections::{BTreeMap, HashMap};

use crate::CommitId;
use crate::commit::Commit;
use crate::graph::row::Row;

/// A stash as `git log` reads its commit, with all its parents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashCommit {
    /// The commit git keeps the stash as: its first parent is the commit it was made on.
    pub commit: Commit,
    /// Its place in the list of stashes: 0 for `stash@{0}`, the newest.
    pub index: usize,
}

/// The rows of the commits with the stashes among them. Each stash keeps only its first parent, the commit it was made
/// on, and goes before the first commit listed that is not newer than it, but never under that commit; a stash whose
/// commit is not listed is left out. Several stashes made on the same commit are listed the newest first.
///
/// Feature: graph.place-stashes
#[must_use]
pub fn place_stashes(commits: &[Row], stashes: &[StashCommit]) -> Vec<Row> {
    let rows_of: HashMap<&CommitId, usize> = commits
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.id().map(|id| (id, index)))
        .collect();
    let mut newest_first: Vec<&StashCommit> = stashes.iter().collect();
    newest_first.sort_by_key(|stash| stash.index);
    let mut placed: BTreeMap<usize, Vec<Row>> = BTreeMap::new();
    for stash in newest_first {
        let Some(base) = stash.commit.parents.first() else {
            continue;
        };
        let Some(&base_row) = rows_of.get(base) else {
            continue;
        };
        let by_date = commits
            .iter()
            .position(|row| {
                row.commit()
                    .is_some_and(|commit| commit.author_date <= stash.commit.author_date)
            })
            .unwrap_or(base_row);
        let row = Row::Stash {
            commit: Commit {
                parents: vec![base.clone()],
                ..stash.commit.clone()
            },
            index: stash.index,
        };
        placed.entry(by_date.min(base_row)).or_default().push(row);
    }
    let mut rows = Vec::with_capacity(commits.len() + stashes.len());
    for (index, row) in commits.iter().enumerate() {
        if let Some(stashes) = placed.remove(&index) {
            rows.extend(stashes);
        }
        rows.push(row.clone());
    }
    rows
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{StashCommit, place_stashes};
    use crate::graph::row::Row;
    use crate::graph::testing::{commit_at, id};

    const DAY: i64 = 86_400;

    fn stash(name: &str, index: usize, base: &str, date: i64) -> StashCommit {
        StashCommit {
            commit: commit_at(name, &[base, &format!("{name}i")], date),
            index,
        }
    }

    fn commit(name: &str, parent: &str, date: i64) -> Row {
        let parents: Vec<&str> = [parent].into_iter().filter(|parent| !parent.is_empty()).collect();
        Row::Commit(commit_at(name, &parents, date))
    }

    /// The names of the rows, as the tests wrote them: the stashes by their place in the list.
    fn order(rows: &[Row]) -> Vec<String> {
        rows.iter()
            .map(|row| match row {
                Row::Stash { index, .. } => format!("stash@{{{index}}}"),
                Row::Commit(commit) => commit.subject.trim_start_matches("Commit ").to_owned(),
                Row::Uncommitted { .. } => String::from("uncommitted"),
            })
            .collect()
    }

    #[test]
    fn graph_place_stashes__a_stash_goes_where_its_date_puts_it_above_its_base() {
        let commits = [
            commit("T2", "T1", 10 * DAY + 2),
            commit("T1", "W", 10 * DAY + 1),
            commit("W", "", 3 * DAY),
        ];
        let rows = place_stashes(&commits, &[stash("S", 0, "W", 9 * DAY)]);
        assert_eq!(order(&rows), ["T2", "T1", "stash@{0}", "W"]);
        let Row::Stash { commit, .. } = &rows[2] else {
            panic!("not a stash: {:?}", rows[2]);
        };
        assert_eq!(commit.parents, [id("W")]);
    }

    #[test]
    fn graph_place_stashes__a_stash_older_than_its_base_goes_right_above_it() {
        let commits = [commit("C2", "C1", 3 * DAY), commit("C1", "", 2 * DAY)];
        let rows = place_stashes(&commits, &[stash("S", 0, "C2", DAY)]);
        assert_eq!(order(&rows), ["stash@{0}", "C2", "C1"]);
    }

    #[test]
    fn graph_place_stashes__two_stashes_on_one_commit_are_listed_the_newest_first() {
        let commits = [commit("C2", "C1", 3 * DAY), commit("C1", "", 2 * DAY)];
        let stashes = [stash("S1", 1, "C1", 4 * DAY), stash("S0", 0, "C1", 5 * DAY)];
        let rows = place_stashes(&commits, &stashes);
        assert_eq!(order(&rows), ["stash@{0}", "stash@{1}", "C2", "C1"]);
        let older = [stash("S1", 1, "C1", DAY), stash("S0", 0, "C1", DAY + 1)];
        assert_eq!(
            order(&place_stashes(&commits, &older)),
            ["C2", "stash@{0}", "stash@{1}", "C1"]
        );
    }

    #[test]
    fn graph_place_stashes__a_stash_on_a_commit_not_listed_is_left_out() {
        let commits = [commit("C2", "C1", 3 * DAY), commit("C1", "", 2 * DAY)];
        let rows = place_stashes(&commits, &[stash("S", 0, "F1", 4 * DAY)]);
        assert_eq!(order(&rows), ["C2", "C1"]);
    }
}
