//! Rewriting the commits under `HEAD`: which commits an interactive rebase can make again, where a rebase stopped to
//! change a commit, and what the confirmation says of a commit pushed already.

use std::collections::HashMap;

mod todo;

pub(crate) use todo::body_of;
pub use todo::{Reword, TodoAction, TodoList, TodoStep, build_todo_list};

use crate::CommitId;
use crate::commit::Commit;
use crate::contract::Text;

/// Why an interactive rebase stopped on a commit, when it stopped to change it rather than on a conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopKind {
    /// To edit the commit: amend it, or commit more after it.
    Edit,
    /// To split the commit into several (`split.commit`).
    Split,
}

/// Where an interactive rebase stopped to change a commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebaseStop {
    /// To edit it, or to split it.
    pub kind: StopKind,
    /// The commit, in full.
    pub commit: CommitId,
    /// The first line of its message.
    pub subject: String,
}

/// What [`find_line_before_head`] looks at: the commits loaded, the one checked out, and the one asked about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineBeforeHeadInput<'a> {
    /// The commits loaded, each with its parents.
    pub commits: &'a [Commit],
    /// The commit checked out; none before the first commit.
    pub head: Option<&'a CommitId>,
    /// The commit an interactive rebase would start at.
    pub commit: &'a CommitId,
}

/// The commits from `HEAD` down to a commit, newest first, both included, when they form a single line of first
/// parents without a merge, all loaded; otherwise none.
///
/// Feature: rebase.find-line-before-head
#[must_use]
pub fn find_line_before_head(input: &LineBeforeHeadInput<'_>) -> Option<Vec<CommitId>> {
    let by_id: HashMap<&CommitId, &Commit> = input.commits.iter().map(|commit| (&commit.id, commit)).collect();
    let mut line = Vec::new();
    let mut current = by_id.get(input.head?).copied()?;
    loop {
        if current.is_merge() {
            return None;
        }
        line.push(current.id.clone());
        if &current.id == input.commit {
            return Some(line);
        }
        current = by_id.get(current.parents.first()?).copied()?;
    }
}

/// How many remote branches the note of a pushed commit names before it counts the others.
const NAMED_BRANCHES: usize = 3;

/// The start of the note of an operation that rewrites a commit pushed already, naming the remote branches that have
/// it, up to three; none when no remote branch has it (`rebase.read-pushed`).
#[must_use]
pub fn pushed_note(commit: &CommitId, remote_branches: &[String]) -> Option<Text> {
    if remote_branches.is_empty() {
        return None;
    }
    let named: Vec<&str> = remote_branches
        .iter()
        .take(NAMED_BRANCHES)
        .map(String::as_str)
        .collect();
    let others = remote_branches.len().saturating_sub(NAMED_BRANCHES);
    let place = if others > 0 {
        format!("{} and {others} more", named.join(", "))
    } else {
        named.join(", ")
    };
    Some(Text::from(format!(
        "{} is pushed already, on {place}: the branch will have to be force pushed, and whoever has the old commits \
         will have to move to the new ones.",
        commit.short()
    )))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{LineBeforeHeadInput, find_line_before_head, pushed_note};
    use crate::commit::Commit;
    use crate::{CommitId, Timestamp};

    fn id(letter: char) -> CommitId {
        let digit = match letter {
            'A' => 'a',
            'B' => 'b',
            'C' => 'c',
            'D' => 'd',
            _ => 'e',
        };
        CommitId::parse(&digit.to_string().repeat(40)).unwrap()
    }

    fn commit(letter: char, parents: &str) -> Commit {
        Commit {
            id: id(letter),
            parents: parents.chars().map(id).collect(),
            author_name: String::new(),
            author_email: String::new(),
            author_date: Timestamp(0),
            subject: String::new(),
        }
    }

    fn line(commits: &[Commit], asked: char) -> Option<String> {
        let head = id('D');
        let input = LineBeforeHeadInput {
            commits,
            head: Some(&head),
            commit: &id(asked),
        };
        find_line_before_head(&input).map(|line| {
            line.iter()
                .map(|commit| commit.as_str().get(..1).unwrap().to_uppercase())
                .collect::<Vec<_>>()
                .join(", ")
        })
    }

    #[test]
    fn rebase_find_line_before_head__follows_first_parents_down_to_the_commit() {
        let commits = [commit('D', "C"), commit('C', "B"), commit('B', "A"), commit('A', "")];
        assert_eq!(line(&commits, 'C').as_deref(), Some("D, C"));
        assert_eq!(line(&commits, 'A').as_deref(), Some("D, C, B, A"));
        assert_eq!(line(&commits, 'D').as_deref(), Some("D"));
        assert_eq!(line(&commits, 'X'), None);
    }

    #[test]
    fn rebase_find_line_before_head__stops_at_a_merge() {
        // `X` is the letter E here: a commit on another branch, merged by `C`.
        let commits = [
            commit('D', "C"),
            commit('C', "BX"),
            commit('B', "A"),
            commit('X', "A"),
            commit('A', ""),
        ];
        assert_eq!(line(&commits, 'D').as_deref(), Some("D"));
        for asked in ['B', 'C', 'X'] {
            assert_eq!(line(&commits, asked), None, "{asked}");
        }
    }

    #[test]
    fn rebase_find_line_before_head__gives_nothing_past_the_commits_loaded() {
        let commits = [commit('D', "C"), commit('C', "B")];
        assert_eq!(line(&commits, 'A'), None);
        let input = LineBeforeHeadInput {
            commits: &commits,
            head: None,
            commit: &id('C'),
        };
        assert_eq!(find_line_before_head(&input), None);
    }

    #[test]
    fn rebase_read_pushed__note_names_up_to_three_branches() {
        let commit = id('A');
        let names = |count: usize| -> Vec<String> { (1..=count).map(|number| format!("origin/b{number}")).collect() };
        let two = pushed_note(&commit, &names(2)).unwrap().to_string();
        assert!(two.starts_with("aaaaaaa is pushed already, on origin/b1, origin/b2: the branch will have to be"));
        let five = pushed_note(&commit, &names(5)).unwrap().to_string();
        assert!(
            five.contains("on origin/b1, origin/b2, origin/b3 and 2 more: "),
            "{five}"
        );
        assert_eq!(pushed_note(&commit, &[]), None);
    }
}
