//! The versions of a file in conflict that the Merge page compares.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;

use model::state::Operation;

use crate::read::conflicts::{ConflictFile, parse_stages, stages_arguments};
use crate::read::fields::unexpected;
use crate::read::repo::{read_location, read_operation};
use crate::{Git, GitError};

/// Which file, and what the File page holds of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeSidesRequest {
    /// Its path from the top of the working tree.
    pub file: String,
    /// The text a File page holds for it when it has edits not saved yet; none when it has none.
    pub unsaved: Option<Vec<u8>>,
}

/// The versions of a file in conflict.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MergeSides {
    /// The common ancestor; empty when the file has none, as when both sides added it.
    pub base: Vec<u8>,
    /// The side checked out; empty when it deleted the file.
    pub current: Vec<u8>,
    /// The side coming in; empty when it deleted the file.
    pub incoming: Vec<u8>,
    /// What the file is now: the edits not saved yet of a File page, or else the file on disk, empty when it is not
    /// there.
    pub result: Vec<u8>,
    /// The operation that stopped on the conflict, for the names of the sides.
    pub operation: Option<Operation>,
    /// Whether the file is in conflict with a version on each side: the index keeps its stages 2 and 3.
    pub has_both_sides: bool,
}

/// The versions the Merge page compares for a file in conflict: the base, the current side and the incoming side from
/// the index, the result from the File page when it has unsaved edits and from the disk otherwise, and the operation
/// in progress.
///
/// Feature: merge-view.read-sides
///
/// # Errors
///
/// When git cannot be started or fails, prints what this does not understand, or the file on disk cannot be read.
pub fn read_sides(git: &Git, request: &MergeSidesRequest) -> Result<MergeSides, GitError> {
    let location = read_location(git)?;
    let git = git.in_directory(&location.folder);
    let stages = parse_stages(&git.run_read(&stages_arguments(std::slice::from_ref(&request.file)))?)?;
    let file = stages.into_iter().find(|file| file.path == request.file);
    let has = |stage: fn(&ConflictFile) -> bool| file.as_ref().is_some_and(stage);
    let base = side(&git, &request.file, Stage::Base, has(|file| file.base.is_some()))?;
    let current = side(&git, &request.file, Stage::Current, has(|file| file.current.is_some()))?;
    let incoming = side(
        &git,
        &request.file,
        Stage::Incoming,
        has(|file| file.incoming.is_some()),
    )?;
    let result = match &request.unsaved {
        Some(text) => text.clone(),
        None => on_disk(&request.file, &location.folder)?,
    };
    Ok(MergeSides {
        base,
        current,
        incoming,
        result,
        operation: read_operation(&location.git_folder),
        has_both_sides: has(|file| file.current.is_some()) && has(|file| file.incoming.is_some()),
    })
}

/// A version the index keeps of a file in conflict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Stage 1, the common ancestor.
    Base,
    /// Stage 2, the side checked out.
    Current,
    /// Stage 3, the side coming in.
    Incoming,
}

/// `git show :<stage>:<file>`: the content of one version of a file in conflict.
#[must_use]
pub fn side_arguments(file: &str, stage: Stage) -> Vec<OsString> {
    let number = match stage {
        Stage::Base => 1,
        Stage::Current => 2,
        Stage::Incoming => 3,
    };
    vec![OsString::from("show"), OsString::from(format!(":{number}:{file}"))]
}

/// The content of a version, empty when the index does not keep it.
fn side(git: &Git, file: &str, stage: Stage, is_kept: bool) -> Result<Vec<u8>, GitError> {
    if is_kept {
        git.run_read(&side_arguments(file, stage))
    } else {
        Ok(Vec::new())
    }
}

/// The file as it is on disk, empty when it is not there: a side deleted it.
fn on_disk(file: &str, top: &std::path::Path) -> Result<Vec<u8>, GitError> {
    let path = file.split('/').fold(top.to_path_buf(), |path, name| path.join(name));
    match fs::read(&path) {
        Ok(content) => Ok(content),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(unexpected(
            "the file in conflict",
            &format!("{}: {error}", path.display()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::{Stage, side_arguments};

    #[test]
    fn asks_for_each_stage_by_its_number() {
        assert_eq!(side_arguments("a b.txt", Stage::Base), ["show", ":1:a b.txt"]);
        assert_eq!(side_arguments("-x", Stage::Current), ["show", ":2:-x"]);
        assert_eq!(side_arguments("x", Stage::Incoming), ["show", ":3:x"]);
    }
}
