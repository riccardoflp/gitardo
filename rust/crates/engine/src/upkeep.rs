//! Answering the queries of the upkeep of a repository, how much room its objects take (`upkeep.read-size`), and of
//! an archive, the folders of a commit (`archive.save`).

use git::{Git, GitError};
use model::contract::{Answer, Query};

/// What a query of ignoring, of the upkeep or of an archive reads, as the log writes it.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::RepositorySize => git::command_line(&git::read::upkeep::size_arguments()),
        Query::CommitFolders { revision } => git::command_line(&folders_arguments(revision)),
        _ => crate::ignoring::describe(query),
    }
}

/// The answer of one of these queries, read now: the size of the repository git runs in, as the numbers of
/// `RepositorySize::to_names`, or the folders of a commit.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    if let Query::CommitFolders { revision } = query {
        let output = git.run_read(&folders_arguments(revision))?;
        let text = String::from_utf8_lossy(&output);
        let folders = text.split('\0').filter(|folder| !folder.is_empty()).map(str::to_owned);
        return Ok(Answer::Names(folders.collect()));
    }
    let request = git::read::upkeep::SizeRequest {
        folder: git.directory().to_path_buf(),
    };
    Ok(Answer::Names(git::read::upkeep::read_size(git, &request)?.to_names()))
}

/// `git ls-tree -r -d --name-only -z <revision>`: the folders of a commit, at any depth.
fn folders_arguments(revision: &str) -> Vec<String> {
    ["ls-tree", "-r", "-d", "--name-only", "-z", revision, "--"]
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect()
}
