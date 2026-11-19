//! The message git prepared for the next commit: what a squash merge leaves in `SQUASH_MSG` while its changes wait to
//! be committed (`commit.prepared-message`); and the diff the draft of a commit message is written from
//! (`commit.read-draft-diff`).

use std::fs;
use std::io;
use std::path::Path;

use model::commit::{Author, SigningConfig, SigningKind, authors_of, is_true};

use crate::read::fields::read_or_nothing;
use crate::read::repo::read_location;
use crate::{Git, GitError};

/// The name of the file git writes the message of a squash merge in, in the git folder of the worktree.
pub const SQUASH_MESSAGE_FILE: &str = "SQUASH_MSG";

/// Which worktree: the one git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PreparedMessageRequest;

/// The message git prepared for the commit of a squash merge in this worktree, without the line breaks and spaces at
/// its end; none when no squash merge waits, or its message is empty.
///
/// Feature: commit.prepared-message
///
/// # Errors
///
/// When the folder is not in a repository, git cannot be started or fails, or the file is there and cannot be read.
pub fn read_prepared_message(git: &Git, _request: &PreparedMessageRequest) -> Result<Option<String>, GitError> {
    let location = read_location(git)?;
    read_message_file(&location.git_folder.join(SQUASH_MESSAGE_FILE))
}

/// What the draft of a commit message describes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct DraftDiffRequest {
    /// Whether something is staged: then only the staged changes are described, as **Commit** would commit them.
    pub is_staged: bool,
}

/// The arguments of the diff a draft is written from: of what is staged when something is, of every change of the
/// tracked files otherwise; without colors and without the external diff program a user may have set up.
#[must_use]
pub fn draft_diff_arguments(request: &DraftDiffRequest) -> Vec<&'static str> {
    let mut arguments = vec!["diff"];
    if request.is_staged {
        arguments.push("--cached");
    }
    arguments.extend(["--no-color", "--no-ext-diff"]);
    arguments
}

/// The diff the draft of a commit message is written from (`commit.draft-message`), as git prints it; bytes that are
/// not UTF-8 are replaced.
///
/// Feature: commit.read-draft-diff
///
/// # Errors
///
/// When the folder is not in a repository, or git cannot be started or fails.
pub fn read_draft_diff(git: &Git, request: &DraftDiffRequest) -> Result<String, GitError> {
    let diff = git.run_read(&draft_diff_arguments(request))?;
    Ok(String::from_utf8_lossy(&diff).into_owned())
}

/// Which configuration: the one git reads in the folder it runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SigningRequest;

/// `git config -z --get-regexp ^(commit\.gpgsign|gpg\.format|user\.signingkey)$`: each key and its value, from every
/// file of the configuration in the order git reads them; the exit code 1 when none is set.
#[must_use]
pub fn signing_arguments(_request: &SigningRequest) -> Vec<&'static str> {
    vec![
        "config",
        "-z",
        "--get-regexp",
        r"^(commit\.gpgsign|gpg\.format|user\.signingkey)$",
    ]
}

/// How git signs commits, from what [`signing_arguments`] printed. A key set in several files takes the value git
/// reads last, as git does; a key with no value at all, printed without a line break, is true.
#[must_use]
pub fn parse_signing(output: &[u8]) -> SigningConfig {
    let text = String::from_utf8_lossy(output);
    let mut signing = SigningConfig::default();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        let (key, value) = match record.split_once('\n') {
            Some((key, value)) => (key, Some(value)),
            None => (record, None),
        };
        match key.to_ascii_lowercase().as_str() {
            "commit.gpgsign" => signing.signs_every_commit = is_true(value),
            "gpg.format" => signing.kind = SigningKind::of_format(value.unwrap_or_default()),
            "user.signingkey" => signing.key = value.map(str::to_owned).filter(|key| !key.is_empty()),
            // The pattern asks for these three keys alone.
            _ => {}
        }
    }
    signing
}

/// Whether git's configuration signs every commit, in which kind and with which key, read with the state of the
/// repository.
///
/// Feature: commit.read-signing
///
/// # Errors
///
/// When git cannot be started or fails otherwise than finding none of the keys.
pub fn read_signing(git: &Git, request: &SigningRequest) -> Result<SigningConfig, GitError> {
    let arguments: Vec<std::ffi::OsString> = signing_arguments(request).into_iter().map(Into::into).collect();
    Ok(read_or_nothing(git, &arguments)?.map_or_else(SigningConfig::default, |output| parse_signing(&output)))
}

/// Which history: every branch, remote branch and tag of the repository git runs in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AuthorsRequest;

/// How many commits are read at most, the newest first, so that a long history answers at once.
pub const AUTHORS_READ: usize = 10_000;

/// `git log -z --format=%aN%x1f%aE --max-count=10000 --branches --remotes --tags`: the name and the email of the author
/// of each commit, the newest first, as the repository's `.mailmap` says them.
#[must_use]
pub fn authors_arguments(_request: &AuthorsRequest) -> Vec<String> {
    vec![
        String::from("log"),
        String::from("-z"),
        String::from("--format=%aN%x1f%aE"),
        format!("--max-count={AUTHORS_READ}"),
        String::from("--branches"),
        String::from("--remotes"),
        String::from("--tags"),
    ]
}

/// The authors of what [`authors_arguments`] printed: one for each email, without regard to case, named as their
/// latest commit says, the one with the most commits first.
#[must_use]
pub fn parse_authors(output: &[u8]) -> Vec<Author> {
    let text = String::from_utf8_lossy(output);
    let commits = text
        .split('\0')
        .map(|record| record.trim_start_matches('\n'))
        .filter_map(|record| record.split_once('\u{1f}'))
        .filter(|(_, email)| !email.is_empty());
    authors_of(commits)
}

/// The people who made commits in the repository, with how many each made, for choosing co-authors.
///
/// Feature: commit.read-authors
///
/// # Errors
///
/// When the folder is not in a repository, or git cannot be started or fails.
pub fn read_authors(git: &Git, request: &AuthorsRequest) -> Result<Vec<Author>, GitError> {
    Ok(parse_authors(&git.run_read(&authors_arguments(request))?))
}

/// The message in a file of the git folder; none when the file is not there or holds only blanks.
///
/// # Errors
///
/// When the file is there and cannot be read.
pub fn read_message_file(path: &Path) -> Result<Option<String>, GitError> {
    match fs::read(path) {
        Ok(bytes) => Ok(parse_message(&bytes)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(source) => Err(GitError::File {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// The message in the bytes of the file, without what ends it; none when nothing is left.
#[must_use]
pub fn parse_message(bytes: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(bytes);
    let message = text.trim_end();
    (!message.is_empty()).then(|| message.to_owned())
}

#[cfg(test)]
mod tests {
    use super::parse_message;

    #[test]
    fn the_end_of_the_message_is_taken_off_and_a_blank_message_is_none() {
        assert_eq!(
            parse_message(b"Squashed commit of the following:\n\n").as_deref(),
            Some("Squashed commit of the following:")
        );
        assert_eq!(parse_message(b"\n \n"), None);
    }
}
