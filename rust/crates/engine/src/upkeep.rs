//! Answering the query of the upkeep of a repository: how much room its objects take (`upkeep.read-size`).

use git::{Git, GitError};
use model::contract::Answer;

/// What a query of ignoring or of the upkeep reads, as the log writes it.
pub(crate) fn describe(query: &model::contract::Query) -> String {
    match query {
        model::contract::Query::RepositorySize => git::command_line(&git::read::upkeep::size_arguments()),
        _ => crate::ignoring::describe(query),
    }
}

/// The size of the repository git runs in, as the numbers of `RepositorySize::to_names`.
pub(crate) fn answer(git: &Git) -> Result<Answer, GitError> {
    let request = git::read::upkeep::SizeRequest {
        folder: git.directory().to_path_buf(),
    };
    Ok(Answer::Names(git::read::upkeep::read_size(git, &request)?.to_names()))
}
