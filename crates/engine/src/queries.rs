//! Answering the queries of features by reading git.

use git::read::reflog::{ReflogRequest, read_entries};
use git::read::search::{RevisionRequest, read_revision};
use git::{Git, GitError};
use model::contract::{Answer, Query};

/// The answer of a query, read now.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::Reflog { reference, count } => {
            let request = ReflogRequest {
                reference: reference.clone(),
                count: *count,
            };
            read_entries(git, &request).map(Answer::Reflog)
        }
        Query::Revision { revision } => {
            let request = RevisionRequest {
                revision: revision.clone(),
            };
            read_revision(git, &request).map(Answer::Commit)
        }
        // The engine answers it from its journal before asking git anything.
        Query::Journal => Ok(Answer::Journal(None)),
        Query::FilesMatch { commit } => {
            let arguments = ["diff", "--quiet", commit.as_str(), "--"];
            let output = git.read_output(&arguments)?;
            match output.code {
                Some(0) => Ok(Answer::Flag(true)),
                Some(1) => Ok(Answer::Flag(false)),
                _ => Err(GitError::Failed {
                    command: git::command_line(&arguments),
                    code: output.code,
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                }),
            }
        }
    }
}

/// The command a query runs, as the log writes it at the debug level.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::Reflog { reference, count } => format!("git log --walk-reflogs --max-count={count} {reference}"),
        Query::FilesMatch { commit } => format!("git diff --quiet {commit} --"),
        Query::Revision { revision } => format!("git rev-parse --verify --quiet {revision}^{{commit}}"),
        Query::Journal => String::from("the journal of Undo"),
    }
}
