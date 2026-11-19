//! Answering the queries of the history of a file and of the blame: the names a file had, where some lines are in the
//! commit checked out, and who last changed each line of a file.

use std::path::PathBuf;

use git::read::blame::{BlameFileRequest, Whitespace, file_arguments, read_file};
use git::read::file_history::{CommittedLinesRequest, NamesRequest, names_arguments, read_committed_lines, read_names};
use git::{Git, GitError};
use model::blame::BlameQuery;
use model::contract::Answer;
use model::file_history::CommittedLinesQuery;

/// Every name a file had, the name given first (`file-history.read-names`).
pub(crate) fn file_names(git: &Git, path: &str) -> Result<Answer, GitError> {
    read_names(git, &NamesRequest { path: path.to_owned() }).map(Answer::Names)
}

/// Where some lines of a file are in the commit checked out (`file-history.read-committed-lines`).
pub(crate) fn committed_lines(git: &Git, query: &CommittedLinesQuery) -> Result<Answer, GitError> {
    read_committed_lines(git, &lines_request(query)).map(Answer::CommittedLines)
}

/// Who last changed each line of a file (`blame.read-file`).
pub(crate) fn blame(git: &Git, query: &BlameQuery) -> Result<Answer, GitError> {
    read_file(git, &blame_request(query)).map(Answer::Blame)
}

/// The answer of a query of the history of a file or of the blame.
pub(crate) fn answer(git: &Git, query: &model::contract::Query) -> Result<Answer, GitError> {
    match query {
        model::contract::Query::FileNames { path } => file_names(git, path),
        model::contract::Query::CommittedLines(lines) => committed_lines(git, lines),
        model::contract::Query::Blame(blame_query) => blame(git, blame_query),
        _ => Ok(Answer::Names(Vec::new())), // Only the queries of the history and of the blame come here.
    }
}

/// The commands a query of the history of a file or of the blame runs, as the log writes them.
pub(crate) fn describe(query: &model::contract::Query) -> String {
    match query {
        model::contract::Query::FileNames { path } => describe_file_names(path),
        model::contract::Query::CommittedLines(lines) => describe_committed_lines(lines),
        model::contract::Query::Blame(blame) => describe_blame(blame),
        _ => String::new(),
    }
}

/// The command that reads the names of a file.
fn describe_file_names(path: &str) -> String {
    git::command_line(&names_arguments(&NamesRequest { path: path.to_owned() }))
}

/// The commands that find some lines in the commit checked out.
fn describe_committed_lines(query: &CommittedLinesQuery) -> String {
    let path = query.renamed_from.as_deref().unwrap_or(&query.path);
    format!("git rev-parse HEAD, git cat-file -e HEAD:{path}, git show HEAD:{path}")
}

/// The command that reads the blame of a file.
fn describe_blame(query: &BlameQuery) -> String {
    git::command_line(&file_arguments(&blame_request(query)))
}

/// The read of the lines of a query.
fn lines_request(query: &CommittedLinesQuery) -> CommittedLinesRequest {
    CommittedLinesRequest {
        path: query.path.clone(),
        renamed_from: query.renamed_from.clone(),
        text: query.text.clone(),
        lines: query.lines,
    }
}

/// The read of the blame of a query.
fn blame_request(query: &BlameQuery) -> BlameFileRequest {
    BlameFileRequest {
        path: PathBuf::from(&query.path),
        whitespace: if query.ignores_whitespace {
            Whitespace::IgnoredWithMoves
        } else {
            Whitespace::Counted
        },
        contents: query.unsaved.clone(),
        revision: query.revision.as_ref().map(|commit| commit.as_str().to_owned()),
    }
}
