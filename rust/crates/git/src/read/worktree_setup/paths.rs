//! What git says of one file or folder chosen from the Files page (`worktree-setup.explain-change`), and which lock
//! files the repository commits at its top (`worktree-setup.propose-commands`).

use std::ffi::OsString;

use model::worktree_setup::{Ignored, Tracked};

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// `git ls-files -z -- :(literal)<path>`: what git tracks of a file or a folder.
#[must_use]
pub fn tracked_arguments(path: &str) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "-z", "--"]);
    words.push(OsString::from(format!(":(literal){path}")));
    words
}

/// `git ls-files --others --ignored --exclude-standard --directory -z -- :(literal)<path>`: what git ignores of a
/// file or a folder, a folder it ignores whole as one entry.
#[must_use]
pub fn ignored_of_arguments(path: &str) -> Vec<OsString> {
    let mut words = arguments(&[
        "ls-files",
        "--others",
        "--ignored",
        "--exclude-standard",
        "--directory",
        "-z",
        "--",
    ]);
    words.push(OsString::from(format!(":(literal){path}")));
    words
}

/// Whether git tracks a file, or files in a folder, from what `git ls-files` printed for it.
#[must_use]
pub fn parse_tracked(path: &str, output: &[u8]) -> Tracked {
    let text = String::from_utf8_lossy(output);
    let mut listed = text.split('\0').filter(|entry| !entry.is_empty());
    match listed.next() {
        None => Tracked::Nothing,
        Some(entry) if entry == path => Tracked::It,
        Some(_) => Tracked::FilesIn,
    }
}

/// How much of a file or a folder git ignores, from what `git ls-files --ignored --directory` printed for it.
#[must_use]
pub fn parse_ignored_of(path: &str, output: &[u8]) -> Ignored {
    let text = String::from_utf8_lossy(output);
    let listed: Vec<&str> = text.split('\0').filter(|entry| !entry.is_empty()).collect();
    if listed.iter().any(|entry| entry.trim_end_matches('/') == path) {
        Ignored::Whole
    } else if listed.is_empty() {
        Ignored::Nothing
    } else {
        Ignored::Part
    }
}

/// Whether git tracks a file or files in a folder, and how much of it git ignores, in the folder git runs in. The
/// path is from the top, with `/`.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_path_status(git: &Git, path: &str) -> Result<(Tracked, Ignored), GitError> {
    let tracked = parse_tracked(path, &git.run_read(&tracked_arguments(path))?);
    let ignored = parse_ignored_of(path, &git.run_read(&ignored_of_arguments(path))?);
    Ok((tracked, ignored))
}

/// `git ls-files -z -- :(top,literal)<file>…`: which of these files the repository commits at its top.
#[must_use]
pub fn committed_at_top_arguments(files: &[&str]) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "-z", "--"]);
    words.extend(files.iter().map(|file| OsString::from(format!(":(top,literal){file}"))));
    words
}

/// Which of these files the repository commits at its top.
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_committed_at_top(git: &Git, files: &[&str]) -> Result<Vec<String>, GitError> {
    let output = git.run_read(&committed_at_top_arguments(files))?;
    Ok(String::from_utf8_lossy(&output)
        .split('\0')
        .filter(|entry| !entry.is_empty())
        .map(str::to_owned)
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{parse_ignored_of, parse_tracked};
    use model::worktree_setup::{Ignored, Tracked};

    #[test]
    fn reads_what_git_tracks_and_ignores_of_a_path() {
        assert_eq!(parse_tracked(".env", b""), Tracked::Nothing);
        assert_eq!(parse_tracked(".env", b".env\0"), Tracked::It);
        assert_eq!(parse_tracked("src", b"src/main.rs\0"), Tracked::FilesIn);
        assert_eq!(parse_ignored_of("node_modules", b"node_modules/\0"), Ignored::Whole);
        assert_eq!(parse_ignored_of("config", b"config/local.json\0"), Ignored::Part);
        assert_eq!(parse_ignored_of("src", b""), Ignored::Nothing);
    }
}
