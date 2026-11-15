//! The message git prepared for the next commit: what a squash merge leaves in `SQUASH_MSG` while its changes wait to
//! be committed (`commit.prepared-message`); and the diff the draft of a commit message is written from
//! (`commit.read-draft-diff`).

use std::fs;
use std::io;
use std::path::Path;

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
