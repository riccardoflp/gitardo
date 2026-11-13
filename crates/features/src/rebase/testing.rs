//! What the tests of the rewrites share: the line read of a commit, made from a few letters.

use model::contract::{Answer, Answers};
use model::rebase::{LineCommit, LineToHead};
use model::state::RepositoryState;

use crate::testing::commit;

/// The line read of a commit with these parents, then the commits from it up to `HEAD`, oldest first, by digit and
/// message.
pub fn line_of(parents: &str, commits: &[(char, &str)]) -> LineToHead {
    LineToHead {
        parents: parents.chars().map(commit).collect(),
        commits: Some(
            commits
                .iter()
                .map(|(digit, message)| LineCommit {
                    id: commit(*digit),
                    message: (*message).to_owned(),
                })
                .collect(),
        ),
    }
}

/// The answers with the line read.
pub fn with_line(answers: Answers, line: LineToHead) -> Answers {
    answers.with(super::LINE, Answer::Line(line))
}

/// The answers with the remote branches that have the commit read.
pub fn with_pushed(answers: Answers, names: &[&str]) -> Answers {
    answers.with(
        super::PUSHED,
        Answer::Names(names.iter().map(|name| (*name).to_owned()).collect()),
    )
}

/// A repository with no commit yet.
pub fn empty() -> RepositoryState {
    let mut state = crate::testing::state();
    state.head.commit = None;
    state
}
