//! The files in conflict, and which of the three versions git keeps in the index each of them has.

use std::ffi::OsString;

use model::CommitId;

use crate::read::fields::{arguments, commit_id, unexpected};
use crate::{Git, GitError};

/// What is being read, for the errors.
const WHAT: &str = "the sides of the files in conflict";

/// Which repository: the one git runs in, at the top of its working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ConflictFilesRequest;

/// The files in conflict.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ConflictFiles {
    /// Each file once, in the order git lists them.
    pub files: Vec<ConflictFile>,
}

/// A file in conflict, and the versions of it the index keeps.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictFile {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Stage 1, the common ancestor; none when both sides added it.
    pub base: Option<StageEntry>,
    /// Stage 2, the side checked out; none when it deleted the file.
    pub current: Option<StageEntry>,
    /// Stage 3, the side coming in; none when it deleted the file.
    pub incoming: Option<StageEntry>,
}

/// One version of a file in conflict, as the index keeps it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageEntry {
    /// Its mode, as git writes it: `100644`.
    pub mode: String,
    /// The object of its content.
    pub object: CommitId,
}

/// The files in conflict, each with the versions the index keeps: the base, the current side and the incoming side.
///
/// Feature: conflicts.read-files
///
/// # Errors
///
/// When git cannot be started, fails, or prints a stage this does not understand.
pub fn read_files(git: &Git, _request: &ConflictFilesRequest) -> Result<ConflictFiles, GitError> {
    let paths = parse_conflict_list(&git.run_read(&conflict_list_arguments())?);
    if paths.is_empty() {
        return Ok(ConflictFiles::default());
    }
    let staged = parse_stages(&git.run_read(&stages_arguments(&paths))?)?;
    let files = paths
        .into_iter()
        .filter_map(|path| staged.iter().find(|file| file.path == path).cloned())
        .collect();
    Ok(ConflictFiles { files })
}

/// `git diff --name-only --diff-filter=U -z`: the files in conflict.
#[must_use]
pub fn conflict_list_arguments() -> Vec<OsString> {
    arguments(&["diff", "--name-only", "--diff-filter=U", "-z"])
}

/// The paths that `git diff --name-only -z` printed, each once.
#[must_use]
pub fn parse_conflict_list(output: &[u8]) -> Vec<String> {
    let mut paths: Vec<String> = Vec::new();
    for path in String::from_utf8_lossy(output)
        .split('\0')
        .filter(|path| !path.is_empty())
    {
        if !paths.iter().any(|known| known == path) {
            paths.push(path.to_owned());
        }
    }
    paths
}

/// `git ls-files --unmerged -z -- <files>`, each file as `:(literal)<file>` so that a name such as `pages/[id].tsx` is
/// that file and not a pattern.
#[must_use]
pub fn stages_arguments(paths: &[String]) -> Vec<OsString> {
    let mut words = arguments(&["ls-files", "--unmerged", "-z", "--"]);
    words.extend(paths.iter().map(|path| OsString::from(format!(":(literal){path}"))));
    words
}

/// The files that `git ls-files --unmerged -z` printed, each with its stages: every record is
/// `<mode> <object> <stage>\t<path>`, and the path is the rest of the record, tabs and line breaks included.
///
/// # Errors
///
/// When a record is not of that form, or its stage is not 1, 2 or 3.
pub fn parse_stages(output: &[u8]) -> Result<Vec<ConflictFile>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut files: Vec<ConflictFile> = Vec::new();
    for record in text.split('\0').filter(|record| !record.is_empty()) {
        let (path, stage, entry) = stage_record(record)?;
        if !files.iter().any(|file| file.path == path) {
            files.push(ConflictFile {
                path: path.to_owned(),
                base: None,
                current: None,
                incoming: None,
            });
        }
        let Some(file) = files.iter_mut().find(|file| file.path == path) else {
            return Err(unexpected(WHAT, record));
        };
        match stage {
            "1" => file.base = Some(entry),
            "2" => file.current = Some(entry),
            "3" => file.incoming = Some(entry),
            _ => return Err(unexpected(WHAT, record)),
        }
    }
    Ok(files)
}

/// The path, the stage and the version of one record.
fn stage_record(record: &str) -> Result<(&str, &str, StageEntry), GitError> {
    let (fields, path) = record.split_once('\t').ok_or_else(|| unexpected(WHAT, record))?;
    let mut fields = fields.split(' ');
    let (Some(mode), Some(object), Some(stage), None) = (fields.next(), fields.next(), fields.next(), fields.next())
    else {
        return Err(unexpected(WHAT, record));
    };
    let entry = StageEntry {
        mode: mode.to_owned(),
        object: commit_id(WHAT, object)?,
    };
    Ok((path, stage, entry))
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{parse_stages, stages_arguments};

    #[test]
    fn conflicts_read_files__a_name_that_is_a_pattern_is_asked_about_literally() {
        let words = stages_arguments(&[String::from("pages/[id].tsx")]);
        assert_eq!(words.last().unwrap(), ":(literal)pages/[id].tsx");
        assert_eq!(words.get(3).unwrap(), "--");
    }

    #[test]
    fn conflicts_read_files__refuses_a_stage_it_does_not_know() {
        let object = "a".repeat(40);
        assert!(parse_stages(format!("100644 {object} 4\tx\0").as_bytes()).is_err());
        assert!(parse_stages(format!("100644 {object} 1 x\0").as_bytes()).is_err());
        let files = parse_stages(format!("100644 {object} 2\ta\tb\0100644 {object} 3\ta\tb\0").as_bytes()).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "a\tb");
        assert!(files[0].base.is_none() && files[0].current.is_some() && files[0].incoming.is_some());
    }
}
