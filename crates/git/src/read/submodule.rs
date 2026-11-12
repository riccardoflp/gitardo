//! The submodules of a repository, and theirs, and where each stands against the commit recorded for it.

mod parse;

use std::path::{Path, PathBuf};
use std::thread;

use model::CommitId;
use model::submodule::{Checkout, Distance, Submodule};

use crate::read::fields::read_or_nothing;
use crate::read::repo::{head_commit_arguments, joined};
use crate::{Git, GitError};

pub use parse::{
    DeclaredSubmodule, checkout_arguments, declared_arguments, distance_arguments, parse_checkout, parse_declared,
    parse_distance, parse_recorded, parse_staged, recorded_arguments, staged_arguments,
};

/// How many levels of submodules of submodules are read.
pub const SUBMODULE_LEVELS: usize = 5;

/// Which repository.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmoduleListRequest {
    /// The top folder of its working tree.
    pub folder: PathBuf,
}

/// The submodules of the repository, and theirs down to [`SUBMODULE_LEVELS`] levels, each followed at once by its own:
/// what `.gitmodules` says of it, the commit recorded for it in the last commit and the one staged when it differs,
/// and for each whose folder has a repository, what it has checked out, its branch, how many of its files are not
/// committed and how far its commit is from the recorded one. Git never runs in a folder with no repository, where it
/// would run in the repository around it. Nothing takes a lock.
///
/// Feature: submodule.read-list
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_list(git: &Git, request: &SubmoduleListRequest) -> Result<Vec<Submodule>, GitError> {
    let mut found = Vec::new();
    let level = Level {
        top: &request.folder,
        prefix: None,
        levels_left: SUBMODULE_LEVELS,
    };
    read_level(git, &level, &mut found)?;
    Ok(found)
}

/// A repository whose submodules are read, and where it is.
struct Level<'a> {
    /// Its top folder.
    top: &'a Path,
    /// Its folder from the top of the repository read; none for that repository.
    prefix: Option<&'a str>,
    /// How many levels may still be read, this one included.
    levels_left: usize,
}

/// Adds the submodules of one repository, each followed by its own.
fn read_level(git: &Git, level: &Level<'_>, found: &mut Vec<Submodule>) -> Result<(), GitError> {
    if level.levels_left == 0 || !level.top.join(".gitmodules").is_file() {
        return Ok(());
    }
    let git = git.in_directory(level.top);
    let declared = parse_declared(&read_or_nothing(&git, &declared_arguments())?.unwrap_or_default());
    if declared.is_empty() {
        return Ok(());
    }
    let folders: Vec<String> = declared.iter().map(|submodule| submodule.folder.clone()).collect();
    let recorded = if read_or_nothing(&git, &head_commit_arguments())?.is_some() {
        parse_recorded(&git.run_read(&recorded_arguments(&folders))?)?
    } else {
        Vec::new()
    };
    let staged = parse_staged(&git.run_read(&staged_arguments(&folders))?)?;
    let checkouts = read_checkouts(&git, level.top, &declared, &recorded)?;
    for (submodule, checkout) in declared.into_iter().zip(checkouts) {
        let full_folder = match level.prefix {
            Some(prefix) => format!("{prefix}/{}", submodule.folder),
            None => submodule.folder.clone(),
        };
        let recorded = commit_of(&recorded, &submodule.folder);
        let staged = commit_of(&staged, &submodule.folder).filter(|commit| Some(commit) != recorded.as_ref());
        let is_initialized = checkout.is_some();
        found.push(Submodule {
            name: submodule.name,
            folder: full_folder.clone(),
            parent: level.prefix.map(str::to_owned),
            url: submodule.url,
            branch: submodule.branch,
            recorded,
            staged,
            checkout,
        });
        if is_initialized {
            let inner = Level {
                top: &folder_path(level.top, &submodule.folder),
                prefix: Some(&full_folder),
                levels_left: level.levels_left - 1,
            };
            read_level(&git, &inner, found)?;
        }
    }
    Ok(())
}

/// What each submodule whose folder has a repository has checked out, all read at once; none for the others.
fn read_checkouts(
    git: &Git,
    top: &Path,
    declared: &[DeclaredSubmodule],
    recorded: &[(String, CommitId)],
) -> Result<Vec<Option<Checkout>>, GitError> {
    thread::scope(|scope| {
        let readers: Vec<_> = declared
            .iter()
            .map(|submodule| {
                let folder = folder_path(top, &submodule.folder);
                let recorded = commit_of(recorded, &submodule.folder);
                folder
                    .join(".git")
                    .exists()
                    .then(|| scope.spawn(move || read_checkout(&git.in_directory(&folder), recorded.as_ref())))
            })
            .collect();
        readers
            .into_iter()
            .map(|reader| reader.map(joined).transpose())
            .collect()
    })
}

/// What the repository of a submodule has checked out, and how far it is from the recorded commit. When git cannot
/// tell how far, because the recorded commit is not in it, that is not known.
fn read_checkout(git: &Git, recorded: Option<&CommitId>) -> Result<Checkout, GitError> {
    let (commit, branch, uncommitted) = parse_checkout(&git.run_read(&checkout_arguments())?)?;
    let distance = match (recorded, &commit) {
        (Some(recorded), Some(commit)) if recorded == commit => Some(Distance::default()),
        (Some(recorded), Some(commit)) => {
            let output = git.read_output(&distance_arguments(recorded, commit))?;
            if output.is_success() {
                Some(parse_distance(&output.stdout)?)
            } else {
                None
            }
        }
        _ => None,
    };
    Ok(Checkout {
        commit,
        branch,
        uncommitted,
        distance,
    })
}

/// The commit of a folder in a list of folders and commits.
fn commit_of(commits: &[(String, CommitId)], folder: &str) -> Option<CommitId> {
    commits
        .iter()
        .find(|(path, _)| path == folder)
        .map(|(_, commit)| commit.clone())
}

/// A folder from the top of a repository, as a path.
fn folder_path(top: &Path, folder: &str) -> PathBuf {
    folder.split('/').fold(top.to_path_buf(), |path, name| path.join(name))
}
