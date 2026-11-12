//! The links in a worktree to folders outside it, which git on Windows would empty when it removes the worktree.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Whether removing a worktree goes through a junction into the folder it links to: on Windows it does, and git
/// deletes what is there; elsewhere git removes the link, not what it leads to.
const REMOVAL_FOLLOWS_LINKS: bool = cfg!(windows);

/// Which worktree to look into.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FolderLinksRequest {
    /// The top folder of the worktree.
    pub worktree: PathBuf,
}

/// The links in a worktree to folders outside it, in full, so that they are taken out before the worktree is removed.
/// Only what git does not track is looked at: the files and folders it does not know and those it ignores. A link
/// is kept when it leads to a folder outside the worktree; a folder is looked into without going past a link. On
/// systems other than Windows it is nothing, and nothing runs.
///
/// Feature: worktree.read-folder-links
///
/// # Errors
///
/// When git cannot be started or fails. A file or a folder that cannot be looked at is passed over.
pub fn read_folder_links(git: &Git, request: &FolderLinksRequest) -> Result<Vec<PathBuf>, GitError> {
    if !REMOVAL_FOLLOWS_LINKS {
        return Ok(Vec::new());
    }
    let git = git.in_directory(&request.worktree);
    let mut entries = Vec::new();
    for words in folder_links_arguments() {
        entries.extend(parse_folder_entries(&git.run_read(&words)?));
    }
    // A worktree whose folder cannot be resolved has no link to tell from it; what the system says comes from the
    // first entry looked at, which fails the same way.
    let Ok(worktree) = fs::canonicalize(&request.worktree) else {
        return Ok(Vec::new());
    };
    let mut links = Vec::new();
    for entry in entries {
        let path = entry
            .split('/')
            .fold(request.worktree.clone(), |path, name| path.join(name));
        collect_links(&path, &worktree, &mut links);
    }
    links.sort();
    links.dedup();
    Ok(links)
}

/// `git ls-files --others --directory -z`, the files and folders git does not know, and
/// `git ls-files --others --ignored --exclude-standard --directory -z`, those it ignores; a folder all of whose content
/// is such is one entry.
#[must_use]
pub fn folder_links_arguments() -> [Vec<OsString>; 2] {
    [
        arguments(&["ls-files", "--others", "--directory", "-z"]),
        arguments(&[
            "ls-files",
            "--others",
            "--ignored",
            "--exclude-standard",
            "--directory",
            "-z",
        ]),
    ]
}

/// The paths that `git ls-files -z` printed, from the top of the worktree, without the `/` git puts after a folder.
#[must_use]
pub fn parse_folder_entries(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .map(|entry| entry.trim_end_matches('/'))
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Adds a path to the links when it is a link to a folder outside the worktree, or what it holds when it is a folder.
fn collect_links(path: &Path, worktree: &Path, links: &mut Vec<PathBuf>) {
    // A path gone since git listed it, or one the system will not show, holds no link to take out.
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return;
    };
    if metadata.file_type().is_symlink() {
        if leads_to_folder_outside(path, worktree) {
            links.push(path.to_path_buf());
        }
    } else if metadata.is_dir() {
        // A folder that cannot be listed is passed over for the same reason.
        let Ok(children) = fs::read_dir(path) else {
            return;
        };
        let mut children: Vec<PathBuf> = children.filter_map(Result::ok).map(|child| child.path()).collect();
        children.sort();
        for child in children {
            collect_links(&child, worktree, links);
        }
    }
}

/// Whether a link leads to a folder that is not inside the worktree. A link to nothing leads nowhere.
fn leads_to_folder_outside(link: &Path, worktree: &Path) -> bool {
    match fs::canonicalize(link) {
        Ok(target) => target.is_dir() && !target.starts_with(worktree),
        Err(_) => false,
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{folder_links_arguments, parse_folder_entries};

    #[test]
    fn worktree_read_folder_links__only_what_git_does_not_track_is_looked_into() {
        for words in folder_links_arguments() {
            assert!(words.iter().any(|word| word == "--others"), "{words:?}");
        }
        assert_eq!(
            parse_folder_entries(b"node_modules/\0packages/api/node_modules\0with space/\0"),
            ["node_modules", "packages/api/node_modules", "with space"]
        );
    }
}
