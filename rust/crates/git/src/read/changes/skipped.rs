//! The tracked files git is told not to look at: marked skip-worktree or assume-unchanged in the index
//! (`changes.read-skipped`).

use std::ffi::OsString;
use std::path::PathBuf;
use std::time::SystemTime;

use model::state::{Skipped, SkippedFile};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Which index to read, and what the read before found in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRequest {
    /// The git folder of the worktree, which holds its index.
    pub git_folder: PathBuf,
    /// What the read before found, with the stamp of the index it read; read again only when the index changed.
    pub before: Option<SkippedRead>,
}

/// The files hidden from git, with the stamp of the index they were read from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedRead {
    /// The size of the index and when it was last written; none when it is not there.
    stamp: Option<(u64, Option<SystemTime>)>,
    /// The files read.
    pub files: Vec<SkippedFile>,
}

/// `git ls-files -v -z`: each file of the index, after a letter that says how git looks at it.
#[must_use]
pub fn skipped_arguments() -> Vec<OsString> {
    arguments(&["ls-files", "-v", "-z"])
}

/// `git config --type=bool --get core.sparseCheckout`: `true` in a sparse checkout.
#[must_use]
pub fn sparse_checkout_arguments() -> Vec<OsString> {
    arguments(&["config", "--type=bool", "--get", "core.sparseCheckout"])
}

/// The files hidden from git, from what `git ls-files -v -z` printed: each record a letter, a space and a path. A letter
/// in lower case is a file assumed unchanged, `S` a file skipped in the working tree, `s` both; any other is a file git
/// looks at. In a sparse checkout the files marked skip-worktree are the checkout's, and are left out.
#[must_use]
pub fn parse_skipped(output: &[u8], is_sparse: bool) -> Vec<SkippedFile> {
    let text = String::from_utf8_lossy(output);
    text.split('\0')
        .filter_map(|record| {
            let (letter, path) = record.split_once(' ')?;
            let mut letters = letter.chars();
            let (Some(letter), None) = (letters.next(), letters.next()) else {
                return None;
            };
            let how = match letter {
                'S' if !is_sparse => Skipped::Worktree,
                's' if is_sparse => Skipped::AssumedUnchanged,
                's' => Skipped::Both,
                'S' => return None,
                lower if lower.is_ascii_lowercase() => Skipped::AssumedUnchanged,
                _ => return None,
            };
            Some(SkippedFile {
                path: path.to_owned(),
                how,
            })
        })
        .collect()
}

/// The tracked files git is told not to look at, by hand; the files the read before found when the index has not
/// changed since, without running git.
///
/// Feature: changes.read-skipped
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_skipped(git: &Git, request: &SkippedRequest) -> Result<SkippedRead, GitError> {
    let stamp = std::fs::metadata(request.git_folder.join("index"))
        .ok()
        .map(|metadata| (metadata.len(), metadata.modified().ok()));
    if let Some(before) = request
        .before
        .as_ref()
        .filter(|before| stamp.is_some() && before.stamp == stamp)
    {
        return Ok(before.clone());
    }
    let sparse = git.read_output(&sparse_checkout_arguments())?;
    let is_sparse = sparse.is_success() && String::from_utf8_lossy(&sparse.stdout).trim() == "true";
    let files = parse_skipped(&git.run_read(&skipped_arguments())?, is_sparse);
    Ok(SkippedRead { stamp, files })
}
