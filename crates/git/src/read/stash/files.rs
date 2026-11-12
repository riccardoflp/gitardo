//! What a stash holds.

use std::ffi::OsString;

use model::stash::StashFiles;

use crate::read::changed_files::parse_name_only;
use crate::read::existing::read_unless_missing;
use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Which stash, by its name: `stash@{0}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StashRef {
    /// Its name, `stash@{N}`.
    pub name: String,
}

/// `git diff --name-only -z <stash>^1 <stash>`: the files it changes against the commit it was made on.
#[must_use]
pub fn stash_changes_arguments(stash: &StashRef) -> Vec<OsString> {
    let parent = format!("{}^1", stash.name);
    arguments(&["diff", "--name-only", "-z", &parent, &stash.name, "--"])
}

/// `git ls-tree -r -z --name-only <stash>^3`: the untracked files it took along, which git keeps in a third parent.
#[must_use]
pub fn stash_untracked_arguments(stash: &StashRef) -> Vec<OsString> {
    arguments(&["ls-tree", "-r", "-z", "--name-only", &untracked_commit(stash), "--"])
}

/// The files a stash changes, and apart from them the untracked files it took along; a stash made without them has
/// none, which is no error.
///
/// Feature: stash.read-files
///
/// # Errors
///
/// Refuses a name that is not of the form `stash@{N}` before git runs, with **"`<name>`" is not a stash.**; fails when
/// git cannot be started or fails.
pub fn read_files(git: &Git, request: &StashRef) -> Result<StashFiles, GitError> {
    if !is_stash_name(&request.name) {
        return Err(GitError::Refused {
            reason: format!("\"{}\" is not a stash.", request.name),
        });
    }
    let changed = parse_name_only(&git.run_read(&stash_changes_arguments(request))?);
    let untracked = read_unless_missing(git, &stash_untracked_arguments(request), &untracked_commit(request))?
        .map(|output| parse_name_only(&output))
        .unwrap_or_default();
    Ok(StashFiles { changed, untracked })
}

/// The commit of the untracked files of a stash: its third parent.
fn untracked_commit(stash: &StashRef) -> String {
    format!("{}^3", stash.name)
}

/// Whether a name is `stash@{N}`, `N` a number.
fn is_stash_name(name: &str) -> bool {
    name.strip_prefix("stash@{")
        .and_then(|rest| rest.strip_suffix('}'))
        .is_some_and(|number| !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit()))
}

#[cfg(test)]
mod tests {
    use super::is_stash_name;

    #[test]
    fn knows_a_stash_by_its_name() {
        for (name, expected) in [
            ("stash@{0}", true),
            ("stash@{12}", true),
            ("stash@{}", false),
            ("stash@{-1}", false),
            ("main", false),
            ("stash@{0}^3", false),
        ] {
            assert_eq!(is_stash_name(name), expected, "{name}");
        }
    }
}
