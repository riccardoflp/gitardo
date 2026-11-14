//! The texts the rules that resolve conflicts are given (`conflicts.resolve-by-rules`): each file in conflict as it is
//! on disk, and the three versions of it the index keeps.

use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::Path;

use model::conflicts::{ConflictTexts, FileStages, MOST_FILES_LOOKED_INTO};

use super::{ConflictFile, ConflictFilesRequest, StageEntry, parse_stages, read_files, stages_arguments};
use crate::{Git, GitError};

/// The files in conflict, the first 200 in the order git lists them, each as it is on disk (none when it is not there
/// or is not a regular file) and in the three versions the index keeps.
///
/// # Errors
///
/// When git cannot be started or fails, or a file on disk cannot be read for another reason than not being there.
pub fn read_texts(git: &Git, request: &ConflictFilesRequest) -> Result<Vec<ConflictTexts>, GitError> {
    let files = read_files(git, request)?;
    files
        .files
        .iter()
        .take(MOST_FILES_LOOKED_INTO)
        .map(|file| {
            Ok(ConflictTexts {
                path: file.path.clone(),
                file: on_disk(git.directory(), &file.path)?,
                base: blob(git, file.base.as_ref())?,
                current: blob(git, file.current.as_ref())?,
                incoming: blob(git, file.incoming.as_ref())?,
            })
        })
        .collect()
}

/// Which versions the index keeps of one file in conflict; none when it is not in conflict.
///
/// # Errors
///
/// When git cannot be started or fails, or prints a stage this does not understand.
pub fn read_stages(git: &Git, path: &str) -> Result<Option<FileStages>, GitError> {
    let staged = parse_stages(&git.run_read(&stages_arguments(&[path.to_owned()]))?)?;
    Ok(staged.iter().find(|file| file.path == path).map(stages_of))
}

/// The versions a file has, as the features see them.
fn stages_of(file: &ConflictFile) -> FileStages {
    FileStages {
        path: file.path.clone(),
        has_base: file.base.is_some(),
        has_current: file.current.is_some(),
        has_incoming: file.incoming.is_some(),
    }
}

/// `git cat-file blob <object>`: the content of a version the index keeps.
#[must_use]
pub fn blob_arguments(entry: &StageEntry) -> Vec<OsString> {
    ["cat-file", "blob", entry.object.as_str()]
        .iter()
        .map(OsString::from)
        .collect()
}

/// The content of a version, none when the index does not keep it.
fn blob(git: &Git, entry: Option<&StageEntry>) -> Result<Option<Vec<u8>>, GitError> {
    entry.map(|entry| git.run_read(&blob_arguments(entry))).transpose()
}

/// The bytes of a file of the working tree, none when it is not there or is not a regular file.
fn on_disk(top: &Path, file: &str) -> Result<Option<Vec<u8>>, GitError> {
    let path = file.split('/').fold(top.to_path_buf(), |path, name| path.join(name));
    let failed = |source| GitError::File {
        path: path.clone(),
        source,
    };
    match fs::symlink_metadata(&path) {
        Ok(metadata) if metadata.is_file() => fs::read(&path).map(Some).map_err(failed),
        Ok(_) => Ok(None),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failed(error)),
    }
}
