//! Answering the query of the notes of commits: the text of the note of a commit (`note.edit`).

use git::{Git, GitError};
use model::contract::{Answer, Query};

/// What a query of the notes reads, as the log writes it.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::Note { commit } => git::command_line(&git::read::note::text_arguments(commit)),
        _ => String::new(), // Only the queries of the notes come here.
    }
}

/// The answer of a query of the notes, read now: the text of a note, empty when the commit has none.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::Note { commit } => Ok(Answer::Text(
            git::read::note::read_text(git, commit)?.unwrap_or_default(),
        )),
        _ => Ok(Answer::Text(String::new())), // Only the queries of the notes come here.
    }
}
