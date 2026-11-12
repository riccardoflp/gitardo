//! What the link and copy patterns of the page take from the checkout, asked of git.

use std::ffi::OsString;
use std::path::PathBuf;

use model::worktree_setup::{
    CopyInput, LinkInput, PatternTake, SetupPreview, folders_holding_matches, folders_to_link, paths_to_copy,
};

use crate::read::fields::{arguments, read_or_nothing};
use crate::read::repo::head_commit_arguments;
use crate::{Git, GitError, command_line};

/// Which checkout, and the patterns of the page.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PatternsRequest {
    /// The top folder of the worktree the views show.
    pub folder: PathBuf,
    /// The patterns of the folders to link, in `.gitignore` syntax.
    pub link_patterns: Vec<String>,
    /// The patterns of the files and folders to copy, in `.gitignore` syntax.
    pub copy_patterns: Vec<String>,
}

/// Whether a listing of what git ignores keeps the folders that hold nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmptyFolders {
    /// Listed, as for the folders to link.
    Listed,
    /// Left out (`--no-empty-directory`), as for the paths to copy.
    LeftOut,
}

/// What git says of the checkout whatever the patterns are.
struct Checkout {
    git: Git,
    ignored_files: Vec<String>,
    whole_for_links: Vec<String>,
    whole_for_copies: Vec<String>,
    commit_files: Vec<String>,
}

/// What the link and copy patterns take from the checkout, as for a worktree that has nothing of them yet: the
/// folders all the link patterns link and the paths all the copy patterns copy, then how many each pattern takes on
/// its own, a copy pattern with the links of all the patterns in place.
///
/// Feature: worktree-setup.read-what-patterns-take
///
/// # Errors
///
/// When git cannot be started or fails.
pub fn read_what_patterns_take(git: &Git, request: &PatternsRequest) -> Result<SetupPreview, GitError> {
    let git = git.in_directory(&request.folder);
    let commit_files = if read_or_nothing(&git, &head_commit_arguments())?.is_some() {
        parse_paths(&git.run_read(&commit_files_arguments())?)
    } else {
        Vec::new()
    };
    let checkout = Checkout {
        ignored_files: parse_paths(&git.run_read(&ignored_files_arguments())?),
        whole_for_links: parse_folders(&git.run_read(&ignored_whole_arguments(EmptyFolders::Listed))?),
        whole_for_copies: parse_folders(&git.run_read(&ignored_whole_arguments(EmptyFolders::LeftOut))?),
        commit_files,
        git,
    };
    let linked = link(&checkout, &request.link_patterns)?;
    let copied = copy(&checkout, &request.copy_patterns, &linked)?;
    let mut link_patterns = Vec::new();
    for pattern in &request.link_patterns {
        let count = link(&checkout, std::slice::from_ref(pattern))?.len();
        link_patterns.push(PatternTake {
            pattern: pattern.clone(),
            count,
        });
    }
    let mut copy_patterns = Vec::new();
    for pattern in &request.copy_patterns {
        let count = copy(&checkout, std::slice::from_ref(pattern), &linked)?.len();
        copy_patterns.push(PatternTake {
            pattern: pattern.clone(),
            count,
        });
    }
    Ok(SetupPreview {
        linked,
        copied,
        link_patterns,
        copy_patterns,
    })
}

/// The folders some link patterns link.
fn link(checkout: &Checkout, patterns: &[String]) -> Result<Vec<String>, GitError> {
    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    let git = &checkout.git;
    let matched_files = parse_paths(&git.run_read(&matched_arguments(patterns, Listing::Files))?);
    let named_folders = parse_folders(&git.run_read(&matched_arguments(patterns, Listing::Folders))?);
    let candidates = folders_holding_matches(&checkout.ignored_files, &matched_files);
    let ignored_folders = if candidates.is_empty() {
        Vec::new()
    } else {
        let words = ignored_folders_arguments();
        let output = git.read_output_with_input(&words, &ignored_folders_input(&candidates))?;
        // `check-ignore` ends with 1 when it ignores none of them.
        match output.code {
            Some(0 | 1) => parse_paths(&output.stdout),
            code => {
                return Err(GitError::Failed {
                    command: command_line(&words),
                    code,
                    stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                });
            }
        }
    };
    Ok(folders_to_link(&LinkInput {
        ignored_files: checkout.ignored_files.clone(),
        ignored_whole: checkout.whole_for_links.clone(),
        matched_files,
        named_folders,
        ignored_folders,
    }))
}

/// The files and folders some copy patterns copy, with these folders linked.
fn copy(checkout: &Checkout, patterns: &[String], linked: &[String]) -> Result<Vec<String>, GitError> {
    if patterns.is_empty() {
        return Ok(Vec::new());
    }
    let matched_files = parse_paths(&checkout.git.run_read(&matched_arguments(patterns, Listing::Files))?);
    Ok(paths_to_copy(&CopyInput {
        ignored_files: checkout.ignored_files.clone(),
        ignored_whole: checkout.whole_for_copies.clone(),
        matched_files,
        linked_folders: linked.to_vec(),
        commit_files: checkout.commit_files.clone(),
    }))
}

/// `git ls-files --others --ignored --exclude-standard -z`: every file git ignores.
#[must_use]
pub fn ignored_files_arguments() -> Vec<OsString> {
    arguments(&["ls-files", "--others", "--ignored", "--exclude-standard", "-z"])
}

/// `git ls-files --others --ignored --exclude-standard --directory -z`, with `--no-empty-directory` when empty
/// folders are left out: the folders git ignores whole, with a `/` after them, and the other files.
#[must_use]
pub fn ignored_whole_arguments(empty_folders: EmptyFolders) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "--others", "--ignored", "--exclude-standard", "--directory"]);
    if empty_folders == EmptyFolders::LeftOut {
        words.push(OsString::from("--no-empty-directory"));
    }
    words.push(OsString::from("-z"));
    words
}

/// What a listing of what the patterns match gives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Listing {
    /// Every file.
    Files,
    /// A folder matched whole as one entry, with a `/` after it.
    Folders,
}

/// `git ls-files --others --ignored --exclude=<pattern>… -z`, with `--directory` for the folders: what the patterns
/// match among what git does not track, the patterns alone and not git's own ignore files.
#[must_use]
pub fn matched_arguments(patterns: &[String], listing: Listing) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "--others", "--ignored"]);
    words.extend(
        patterns
            .iter()
            .map(|pattern| OsString::from(format!("--exclude={pattern}"))),
    );
    if listing == Listing::Folders {
        words.push(OsString::from("--directory"));
    }
    words.push(OsString::from("-z"));
    words
}

/// `git check-ignore --no-index -z --stdin`: which of the folders given git ignores.
#[must_use]
pub fn ignored_folders_arguments() -> Vec<OsString> {
    arguments(&["check-ignore", "--no-index", "-z", "--stdin"])
}

/// The folders for `check-ignore`, each ended by a NUL.
#[must_use]
pub fn ignored_folders_input(folders: &[String]) -> Vec<u8> {
    folders
        .iter()
        .flat_map(|folder| format!("{folder}\0").into_bytes())
        .collect()
}

/// `git ls-tree -r -z --name-only HEAD`: the files of the commit checked out.
#[must_use]
pub fn commit_files_arguments() -> Vec<OsString> {
    arguments(&["ls-tree", "-r", "-z", "--name-only", "HEAD"])
}

/// The paths a `-z` listing printed, folders without the `/` after them.
#[must_use]
pub fn parse_paths(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .map(|path| path.trim_end_matches('/'))
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

/// The folders of a `--directory` listing, the entries with a `/` after them, without it.
#[must_use]
pub fn parse_folders(output: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(output)
        .split('\0')
        .filter_map(|path| path.strip_suffix('/'))
        .map(str::to_owned)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Listing, matched_arguments, parse_folders, parse_paths};

    #[test]
    fn reads_folders_and_paths_and_gives_each_pattern_as_an_exclude() {
        assert_eq!(parse_folders(b"node_modules/\0.env\0a b/\0"), ["node_modules", "a b"]);
        assert_eq!(parse_paths(b"node_modules/\0.env\0"), ["node_modules", ".env"]);
        let words = matched_arguments(&[String::from("-x"), String::from("/.env")], Listing::Folders);
        let words: Vec<&str> = words.iter().map(|word| word.to_str().unwrap()).collect();
        assert_eq!(
            words,
            [
                "ls-files",
                "--others",
                "--ignored",
                "--exclude=-x",
                "--exclude=/.env",
                "--directory",
                "-z"
            ]
        );
    }
}
