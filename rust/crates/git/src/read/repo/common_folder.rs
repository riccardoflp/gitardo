//! The git folder every worktree of a repository shares, where its references live (`repo.common-folder`).

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

use crate::read::fields::{arguments, unexpected};
use crate::{Git, GitError};

/// Which repository: the one git runs in, from any folder of its working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CommonFolderRequest;

/// The git folder shared by every worktree of the repository, in full: the `.git` of the main worktree, whichever
/// worktree or folder of it git runs in.
///
/// Feature: repo.common-folder
///
/// # Errors
///
/// When the folder is not in a repository, or git cannot be started or fails.
pub fn read_common_folder(git: &Git, _request: &CommonFolderRequest) -> Result<PathBuf, GitError> {
    parse_common_folder(&git.run_read(&common_folder_arguments())?, git.directory())
}

/// `git rev-parse --git-common-dir`, which prints the folder from the folder git runs in, or in full.
#[must_use]
pub fn common_folder_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--git-common-dir"])
}

/// The folder that [`common_folder_arguments`] printed, in full, taken from the folder git ran in when git gave it from
/// there, without the `.` and `..` of its path.
///
/// # Errors
///
/// When git printed nothing.
pub fn parse_common_folder(output: &[u8], ran_in: &Path) -> Result<PathBuf, GitError> {
    let text = String::from_utf8_lossy(output);
    let printed = text.trim_end_matches(['\n', '\r']);
    if printed.is_empty() {
        return Err(unexpected("the common git folder", &text));
    }
    Ok(without_dots(&ran_in.join(printed)))
}

/// A path without its `.` and with each `..` taken with the name before it, as the system would read it; a link is
/// not followed.
fn without_dots(path: &Path) -> PathBuf {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                let _went_up = clean.pop();
            }
            other => clean.push(other),
        }
    }
    clean
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::parse_common_folder;

    #[test]
    fn a_folder_given_from_where_git_ran_is_made_whole_and_one_given_whole_is_kept() {
        let ran_in = Path::new("/work/app/src");
        assert_eq!(
            parse_common_folder(b"../.git\n", ran_in).unwrap(),
            Path::new("/work/app/.git")
        );
        assert_eq!(
            parse_common_folder(b"/work/main/.git\n", Path::new("/work/feature")).unwrap(),
            Path::new("/work/main/.git")
        );
        assert!(parse_common_folder(b"\n", ran_in).is_err());
    }
}
