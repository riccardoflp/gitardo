//! Whether the history of a repository is shallow, and where it ends: the file `shallow` of the common git folder, which
//! holds the hash of each commit whose parents were not fetched (`repo.read-shallow`).

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use model::CommitId;
use model::state::Shallow;

use crate::read::fields::arguments;
use crate::{Git, GitError};

/// Which repository to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShallowRequest {
    /// The folder git runs in, which the path git prints is relative to.
    pub folder: PathBuf,
}

/// `git rev-parse --is-shallow-repository --git-path shallow`: `true` or `false` on the first line, the path of the file
/// `shallow` on the second, which is in the common git folder, so that every worktree reads the same.
#[must_use]
pub fn shallow_arguments() -> Vec<OsString> {
    arguments(&["rev-parse", "--is-shallow-repository", "--git-path", "shallow"])
}

/// The file `shallow`, when the first line says the repository is shallow; none when it is not.
///
/// # Errors
///
/// When the first line is neither `true` nor `false`, or the second is missing.
pub fn parse_shallow_location(output: &[u8]) -> Result<Option<PathBuf>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut lines = text.lines().map(str::trim);
    let is_shallow = match lines.next() {
        Some("true") => true,
        Some("false") => false,
        other => {
            return Err(GitError::Unexpected {
                what: "whether the repository is shallow",
                detail: format!("{other:?}"),
            });
        }
    };
    let Some(path) = lines.next().filter(|path| !path.is_empty()) else {
        return Err(GitError::Unexpected {
            what: "the path of the file shallow",
            detail: text.to_string(),
        });
    };
    Ok(is_shallow.then(|| PathBuf::from(path)))
}

/// The commits the file `shallow` lists, one hash a line; a line that is not a hash is left out.
#[must_use]
pub fn parse_shallow_file(text: &str) -> Shallow {
    Shallow {
        ends: text.lines().filter_map(|line| CommitId::parse(line.trim())).collect(),
    }
}

/// Whether the repository is shallow and where its history ends; the file is read only when git says it is shallow.
/// A file gone by the time it is read, as when a fetch took the history whole meanwhile, says it is not.
///
/// Feature: repo.read-shallow
///
/// # Errors
///
/// When git cannot be started or fails, or prints what this does not understand.
pub fn read_shallow(git: &Git, request: &ShallowRequest) -> Result<Shallow, GitError> {
    let output = git.run_read(&shallow_arguments())?;
    let Some(path) = parse_shallow_location(&output)? else {
        return Ok(Shallow::default());
    };
    let path = if path.is_absolute() {
        path
    } else {
        request.folder.join(path)
    };
    Ok(read_file(&path))
}

/// The commits of the file `shallow`, none when it cannot be read.
fn read_file(path: &Path) -> Shallow {
    std::fs::read_to_string(path).map_or_else(|_| Shallow::default(), |text| parse_shallow_file(&text))
}

#[cfg(test)]
mod tests {
    #![allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]

    use super::{parse_shallow_file, parse_shallow_location};

    #[test]
    fn repo_read_shallow__the_file_is_read_only_when_git_says_the_repository_is_shallow() {
        assert_eq!(
            parse_shallow_location(b"true\n.git/shallow\n").unwrap(),
            Some(std::path::PathBuf::from(".git/shallow"))
        );
        assert_eq!(parse_shallow_location(b"false\n.git/shallow\n").unwrap(), None);
        assert!(parse_shallow_location(b"maybe\n").is_err());
        let hash = "0123456789abcdef0123456789abcdef01234567";
        assert_eq!(parse_shallow_file(&format!("{hash}\n")).ends.len(), 1);
    }
}
