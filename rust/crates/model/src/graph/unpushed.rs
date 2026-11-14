//! The commits of local branches that are not in their upstream yet: what the next push of each branch sends.

use std::collections::HashMap;

use crate::CommitId;
use crate::graph::reach::Ancestry;
use crate::graph::row::Row;
use crate::state::{Branch, RemoteBranch};

/// The commits among the rows that a push has not sent yet, each with the name of the upstream it would go to
/// (`origin/main`).
///
/// For each local branch whose upstream is not gone, that git counts ahead of it, and whose tip is among the rows: the
/// commits it reaches among the rows less those its upstream reaches. When the upstream's commit is not among the rows,
/// it is further down than all of them, and the commits the branch reaches are the ones not pushed, but only when they
/// are no more than git counts ahead: more would mean the graph leaves the upstream out, and none is marked. A commit
/// two branches have is marked for the first.
///
/// Feature: graph.unpushed
#[must_use]
pub fn unpushed(rows: &[Row], branches: &[Branch], remote_branches: &[RemoteBranch]) -> HashMap<CommitId, String> {
    let is_ahead = |branch: &Branch| branch.upstream.as_ref().is_some_and(|upstream| upstream.ahead > 0);
    if !branches.iter().any(is_ahead) {
        return HashMap::new();
    }
    let ancestry = Ancestry::new(rows);
    let mut marked = HashMap::new();
    for branch in branches {
        let Some(upstream) = branch
            .upstream
            .as_ref()
            .filter(|upstream| !upstream.is_gone && upstream.ahead > 0)
        else {
            continue;
        };
        let Some(tip) = ancestry.row_of(&branch.commit) else {
            continue;
        };
        let name = upstream
            .reference
            .strip_prefix("refs/remotes/")
            .unwrap_or(&upstream.reference);
        let upstream_row = remote_branches
            .iter()
            .find(|remote_branch| remote_branch.name == name)
            .and_then(|remote_branch| ancestry.row_of(&remote_branch.commit));
        let mut not_pushed = ancestry.reached_from(&[tip]);
        if let Some(row) = upstream_row {
            for (wanted, pushed) in not_pushed.iter_mut().zip(ancestry.reached_from(&[row])) {
                *wanted = *wanted && !pushed;
            }
        } else if not_pushed.iter().filter(|wanted| **wanted).count()
            > usize::try_from(upstream.ahead).unwrap_or(usize::MAX)
        {
            continue;
        }
        let commits = rows.iter().zip(not_pushed).filter(|(_, wanted)| *wanted);
        for id in commits.filter_map(|(row, _)| row.id()) {
            let _ = marked.entry(id.clone()).or_insert_with(|| name.to_owned());
        }
    }
    marked
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::unpushed;
    use crate::graph::testing::{branch, id, remote_branch, rows};
    use crate::state::Branch;

    /// `main` following `origin/main`, so many commits ahead of it.
    fn main_ahead(commit: &str, ahead: u32) -> Branch {
        let mut main = branch("main", commit, Some(("origin", "main")));
        if let Some(upstream) = main.upstream.as_mut() {
            upstream.ahead = ahead;
        }
        main
    }

    /// The commits marked, by name, with the upstream each is marked for.
    fn marked(found: &std::collections::HashMap<crate::CommitId, String>, names: &[&str]) -> Vec<Option<String>> {
        names.iter().map(|name| found.get(&id(name)).cloned()).collect()
    }

    #[test]
    fn graph_unpushed__the_commits_ahead_of_the_upstream_are_marked_with_its_name() {
        let rows = rows(&["C2: C1", "C1: B", "B: A", "A:"]);
        let found = unpushed(&rows, &[main_ahead("C2", 2)], &[remote_branch("origin/main", "B")]);
        let upstream = Some(String::from("origin/main"));
        assert_eq!(
            marked(&found, &["C2", "C1", "B", "A"]),
            [upstream.clone(), upstream, None, None]
        );
    }

    #[test]
    fn graph_unpushed__a_branch_with_no_upstream_has_none() {
        let rows = rows(&["C2: C1", "C1:"]);
        assert!(unpushed(&rows, &[branch("main", "C2", None)], &[]).is_empty());
    }

    #[test]
    fn graph_unpushed__narrowed_to_the_local_branches_the_upstream_left_out_marks_none() {
        let rows = rows(&["C2: C1", "C1: B", "B: A", "A:"]);
        let found = unpushed(&rows, &[main_ahead("C2", 2)], &[remote_branch("origin/main", "R")]);
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn graph_unpushed__after_a_push_none_is_marked() {
        let rows = rows(&["C2: C1", "C1: B", "B:"]);
        let found = unpushed(&rows, &[main_ahead("C2", 0)], &[remote_branch("origin/main", "C2")]);
        assert!(found.is_empty());
    }

    #[test]
    fn graph_unpushed__an_upstream_further_down_than_the_rows_marks_what_git_counts_ahead() {
        let rows = rows(&["C2: C1", "C1: B"]);
        let found = unpushed(&rows, &[main_ahead("C2", 2)], &[remote_branch("origin/main", "B")]);
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn graph_unpushed__a_commit_two_branches_have_is_marked_for_the_first() {
        let rows = rows(&["F: C1", "C1: B", "B:"]);
        let mut feature = branch("feature", "F", Some(("origin", "feature")));
        if let Some(upstream) = feature.upstream.as_mut() {
            upstream.ahead = 2;
        }
        let remotes = [remote_branch("origin/main", "B"), remote_branch("origin/feature", "B")];
        let found = unpushed(&rows, &[main_ahead("C1", 1), feature], &remotes);
        assert_eq!(found[&id("C1")], "origin/main");
        assert_eq!(found[&id("F")], "origin/feature");
    }
}
