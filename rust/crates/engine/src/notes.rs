//! Answering the queries of the notes of commits: the text of the note of a commit (`note.edit`), and whether a remote
//! has a reference of notes (`note.fetch`).

use git::{Git, GitError};
use model::contract::{Answer, Query};

/// What a query of the notes reads, as the log writes it.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::Note { commit } => git::command_line(&git::read::note::text_arguments(commit)),
        Query::RemoteNotes { remote, reference } => git::command_line(&remote_arguments(remote, reference)),
        _ => String::new(), // Only the queries of the notes come here.
    }
}

/// The answer of a query of the notes, read now: the text of a note, empty when the commit has none, or whether the
/// remote lists the reference of notes.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::Note { commit } => Ok(Answer::Text(
            git::read::note::read_text(git, commit)?.unwrap_or_default(),
        )),
        Query::RemoteNotes { remote, reference } => {
            let listed = git.run_read(&remote_arguments(remote, reference))?;
            Ok(Answer::Flag(!listed.is_empty()))
        }
        _ => Ok(Answer::Flag(false)), // Only the queries of the notes come here.
    }
}

/// `git ls-remote <remote> <reference>`: the reference of notes of the remote, nothing when it has none.
fn remote_arguments(remote: &str, reference: &str) -> Vec<String> {
    ["ls-remote", remote, reference]
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect()
}
