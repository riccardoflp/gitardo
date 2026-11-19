//! Answering the queries of a repository made from the window (`repo.clone`, `repo.init`): what a folder holds, the
//! repository a folder is in, and the first branch git's configuration gives a new one.

use std::path::Path;

use git::{Git, GitError};
use model::contract::{Answer, Query};

/// The answer of one of these queries, read now.
pub(crate) fn answer(git: &Git, query: &Query) -> Result<Answer, GitError> {
    match query {
        Query::TakenNames { folder } => Ok(Answer::Names(taken_names(folder))),
        Query::RepositoryAround { folder } => repository_around(git, folder),
        Query::DefaultBranch { folder } => default_branch(git, folder),
        _ => Ok(Answer::Names(Vec::new())), // The engine sends only these here.
    }
}

/// What one of these queries reads, as the log writes it.
pub(crate) fn describe(query: &Query) -> String {
    match query {
        Query::GitFolder { folder } => format!("git -C {} rev-parse --absolute-git-dir", folder.display()),
        Query::HasFiles { path: folder } | Query::TakenNames { folder } => format!("look in {}", folder.display()),
        Query::RepositoryAround { folder } => format!("git -C {} rev-parse --show-toplevel", folder.display()),
        Query::DefaultBranch { folder } => format!("git -C {} config --get init.defaultBranch", folder.display()),
        _ => String::new(), // The engine sends only these here.
    }
}

/// The names of what is in a folder and is not an empty folder, sorted; none for a folder that is not there.
fn taken_names(folder: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        // A folder that is not there, or cannot be read, takes no name: git says so when it runs.
        return Vec::new();
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter(|entry| crate::readings::has_files(&entry.path()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// The top folder of the repository a folder is in, if any.
fn repository_around(git: &Git, folder: &Path) -> Result<Answer, GitError> {
    if !folder.is_dir() {
        return Ok(Answer::Paths(Vec::new()));
    }
    let output = git
        .in_directory(folder)
        .read_output(&["rev-parse", "--show-toplevel"])?;
    let top = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let around = if output.is_success() && !top.is_empty() {
        vec![std::path::PathBuf::from(top)]
    } else {
        Vec::new()
    };
    Ok(Answer::Paths(around))
}

/// The first branch `init.defaultBranch` names, if any.
fn default_branch(git: &Git, folder: &Path) -> Result<Answer, GitError> {
    let directory = if folder.is_dir() { folder } else { git.directory() };
    let output = git
        .in_directory(directory)
        .read_output(&["config", "--get", "init.defaultBranch"])?;
    let name = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let names = if output.is_success() && !name.is_empty() {
        vec![name]
    } else {
        Vec::new()
    };
    Ok(Answer::Names(names))
}
