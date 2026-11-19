//! The notes of commits: which commits have one in the reference of notes git uses, and the text of the note of one
//! commit (`note.read`).

use std::collections::BTreeMap;
use std::ffi::OsString;

use model::CommitId;
use model::note::Notes;

use crate::read::fields::{arguments, commit_id, read_or_nothing, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the notes of the commits";

/// What the read before found; the list is read again only when the reference of notes moved since.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct NoteRequest {
    /// The notes the read before found, none on the first read and on **Refresh**.
    pub before: Option<Notes>,
}

/// `git notes get-ref`: the reference of notes git uses, `refs/notes/commits` unless `core.notesRef` or
/// `GIT_NOTES_REF` names another.
#[must_use]
pub fn reference_arguments() -> Vec<OsString> {
    arguments(&["notes", "get-ref"])
}

/// `git rev-parse --verify --quiet <reference>^{commit}`: the commit the reference of notes is at, nothing while it
/// is not there.
#[must_use]
pub fn tip_arguments(reference: &str) -> Vec<OsString> {
    let commit = format!("{reference}^{{commit}}");
    arguments(&["rev-parse", "--verify", "--quiet", &commit])
}

/// `git notes --ref=<reference> list`: a line `<id of the note> <id of the commit>` for each commit with a note.
#[must_use]
pub fn list_arguments(reference: &str) -> Vec<OsString> {
    let reference = format!("--ref={reference}");
    arguments(&["notes", &reference, "list"])
}

/// `git notes show <hash>`: the text of the note of a commit, in the reference git uses.
#[must_use]
pub fn text_arguments(commit: &CommitId) -> Vec<OsString> {
    arguments(&["notes", "show", commit.as_str()])
}

/// The commits with a note, each with the object of its note, from what `git notes list` printed.
///
/// # Errors
///
/// When a line is not two hashes.
pub fn parse_list(output: &[u8]) -> Result<BTreeMap<CommitId, CommitId>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut noted = BTreeMap::new();
    for line in text.lines().filter(|line| !line.is_empty()) {
        let Some((note, commit)) = line.split_once(' ') else {
            return Err(unexpected(WHAT, line));
        };
        let _ = noted.insert(commit_id(WHAT, commit)?, commit_id(WHAT, note)?);
    }
    Ok(noted)
}

/// The text of a note as it was written, its lines kept, without the line break git ends it with.
#[must_use]
pub fn parse_text(output: &[u8]) -> String {
    String::from_utf8_lossy(output)
        .trim_end_matches(['\n', '\r'])
        .to_owned()
}

/// The commits with a note in the reference of notes git uses (Q1210), with the commit that reference is at; the list
/// the read before found when the reference has not moved since, without running `git notes list`. With no reference
/// of notes, there are none.
///
/// Feature: note.read
///
/// # Errors
///
/// When git cannot be started or fails, or prints what this does not understand.
pub fn read(git: &Git, request: &NoteRequest) -> Result<Notes, GitError> {
    let reference = String::from_utf8_lossy(&git.run_read(&reference_arguments())?)
        .trim()
        .to_owned();
    let tip = match read_or_nothing(git, &tip_arguments(&reference))? {
        Some(output) => Some(commit_id(WHAT, String::from_utf8_lossy(&output).trim())?),
        None => None,
    };
    if let Some(before) = request
        .before
        .as_ref()
        .filter(|before| before.reference == reference && before.tip == tip)
    {
        return Ok(before.clone());
    }
    let noted = match tip {
        Some(_) => parse_list(&git.run_read(&list_arguments(&reference))?)?,
        None => BTreeMap::new(),
    };
    Ok(Notes { reference, tip, noted })
}

/// The text of the note of a commit, as it was written; none when it has no note.
///
/// # Errors
///
/// When git cannot be started or fails otherwise than by finding no note.
pub fn read_text(git: &Git, commit: &CommitId) -> Result<Option<String>, GitError> {
    Ok(read_or_nothing(git, &text_arguments(commit))?.map(|output| parse_text(&output)))
}
