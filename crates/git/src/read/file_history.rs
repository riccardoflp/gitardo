//! The history of a file: the names it had, and where some of its lines are in the commit checked out.

use std::ffi::OsString;

use model::CommitId;
use model::file_history::{CommittedLines, HistoryOf, LineRange, MapLinesInput, map_lines};

use crate::read::fields::{arguments, read_or_nothing, unexpected};
use crate::read::repo::{head_commit_arguments, parse_head_commit};
use crate::{Git, GitError};

/// The file whose names to read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamesRequest {
    /// Its name now, from the top of the working tree.
    pub path: String,
}

/// What a commit did to the file, as `git log --follow --name-status` lists it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameChange {
    /// The file was added under this name.
    Added(String),
    /// The file was made as a copy of another, under this name.
    Copied(String),
    /// The file was renamed.
    Renamed {
        /// Its name before.
        from: String,
        /// Its name after.
        to: String,
    },
    /// The file was changed or deleted under this name.
    Changed(String),
}

/// Every name a file had in the history of what is checked out, the name given first, then each older name once,
/// newest first. The history stops at the commit that added the file, or made it as a copy of another: what `--follow`
/// finds before that is another file, with its own history.
///
/// Feature: file-history.read-names
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_names(git: &Git, request: &NamesRequest) -> Result<Vec<String>, GitError> {
    if read_or_nothing(git, &head_commit_arguments())?.is_none() {
        return Ok(vec![request.path.clone()]);
    }
    let changes = parse_name_changes(&git.run_read(&names_arguments(request))?)?;
    Ok(names(&request.path, &changes))
}

/// `git log --follow -M --name-status -z --format= -- :(literal)<file>`: the changes of the file, newest first.
#[must_use]
pub fn names_arguments(request: &NamesRequest) -> Vec<OsString> {
    let literal = format!(":(literal){}", request.path);
    arguments(&[
        "log",
        "--follow",
        "-M",
        "--name-status",
        "-z",
        "--format=",
        "--",
        &literal,
    ])
}

/// The changes that [`names_arguments`] listed: a status, then one name, or two for a rename or a copy, each ended by
/// a NUL.
///
/// # Errors
///
/// When a status is not followed by its names.
pub fn parse_name_changes(output: &[u8]) -> Result<Vec<NameChange>, GitError> {
    let text = String::from_utf8_lossy(output);
    let mut fields = text
        .split('\0')
        .map(|field| field.trim_start_matches('\n'))
        .filter(|field| !field.is_empty());
    let mut changes = Vec::new();
    while let Some(status) = fields.next() {
        let mut name = || {
            fields
                .next()
                .map(str::to_owned)
                .ok_or_else(|| unexpected("the names of a file", status))
        };
        let change = match status.as_bytes().first() {
            Some(b'A') => NameChange::Added(name()?),
            Some(b'C') => {
                let _from = name()?;
                NameChange::Copied(name()?)
            }
            Some(b'R') => NameChange::Renamed {
                from: name()?,
                to: name()?,
            },
            _ => NameChange::Changed(name()?),
        };
        changes.push(change);
    }
    Ok(changes)
}

/// The names of a file from its changes, newest first: the name given, then the name of each change, and the old name
/// of a rename, each once, up to the first addition or copy.
#[must_use]
pub fn names(path: &str, changes: &[NameChange]) -> Vec<String> {
    let mut found = vec![path.to_owned()];
    let mut add = |name: &str| {
        if !found.iter().any(|known| known == name) {
            found.push(name.to_owned());
        }
    };
    for change in changes {
        match change {
            NameChange::Added(_) | NameChange::Copied(_) => break,
            NameChange::Renamed { from, to } => {
                add(to);
                add(from);
            }
            NameChange::Changed(name) => add(name),
        }
    }
    found
}

/// Some lines of a file as the File page has it, to find in the commit checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedLinesRequest {
    /// The file, from the top of the working tree.
    pub path: String,
    /// The name the file had before, when the index has it as a rename: the name the commit has.
    pub renamed_from: Option<String>,
    /// The text of the file as the File page has it, saved or not.
    pub text: String,
    /// The lines of that text.
    pub lines: LineRange,
}

/// Where some lines of a file as it is now are in the commit checked out: the commit, the name of the file there, and
/// the lines they come from, or none when all were added since; or no commit has the file.
///
/// Feature: file-history.read-committed-lines
///
/// # Errors
///
/// When git cannot be started, fails, or prints what this does not understand.
pub fn read_committed_lines(git: &Git, request: &CommittedLinesRequest) -> Result<CommittedLines, GitError> {
    let Some(head) = read_or_nothing(git, &head_commit_arguments())? else {
        return Ok(CommittedLines::InNoCommit);
    };
    let commit = parse_head_commit(&head)?;
    let path = request.renamed_from.as_deref().unwrap_or(&request.path);
    if !git
        .read_output(&committed_file_exists_arguments(&commit, path))?
        .is_success()
    {
        return Ok(CommittedLines::InNoCommit);
    }
    let committed = git.run_read(&committed_file_arguments(&commit, path))?;
    let committed = String::from_utf8_lossy(&committed);
    let input = MapLinesInput {
        committed: &committed,
        current: &request.text,
        lines: request.lines,
    };
    let lines = map_lines(&input);
    Ok(CommittedLines::Committed {
        commit,
        path: path.to_owned(),
        lines,
    })
}

/// What narrows the history the timeline lists to the history of a file, of some lines or of a folder, after the
/// branches it shows: `--parents`, which makes each commit's parents the nearest commits listed so that the lines of
/// the graph stay connected, then `-- :(literal)<name>…` for a file under each of its names or for a folder, or
/// `--no-patch -L<start>,<end>:<path>` for some lines, which git follows from the commit given in place of the branches.
#[must_use]
pub fn history_arguments(history: &HistoryOf) -> Vec<OsString> {
    match history {
        HistoryOf::Lines { path, lines, .. } => {
            let range = format!("-L{},{}:{path}", lines.start, lines.end);
            arguments(&["--parents", "--no-patch", &range])
        }
        HistoryOf::File { .. } | HistoryOf::Folder { .. } => {
            let mut words = arguments(&["--parents", "--"]);
            words.extend(
                history
                    .paths()
                    .into_iter()
                    .map(|path| OsString::from(format!(":(literal){path}"))),
            );
            words
        }
    }
}

/// `git cat-file -e <commit>:<file>`, which ends with the exit code 0 when the commit has the file.
#[must_use]
pub fn committed_file_exists_arguments(commit: &CommitId, path: &str) -> Vec<OsString> {
    arguments(&["cat-file", "-e", &format!("{commit}:{path}")])
}

/// `git show <commit>:<file>`: the file as the commit has it.
#[must_use]
pub fn committed_file_arguments(commit: &CommitId, path: &str) -> Vec<OsString> {
    arguments(&["show", &format!("{commit}:{path}")])
}

#[cfg(test)]
mod tests {
    use super::{NameChange, names, parse_name_changes};

    #[test]
    fn reads_a_rename_a_copy_and_an_addition() {
        let output = b"C058\0src/b.rs\0x.rs\0R055\0src/a b.rs\0src/b.rs\0M\0src/a b.rs\0A\0src/a b.rs\0";
        let changes = parse_name_changes(output).unwrap();
        assert_eq!(
            changes,
            vec![
                NameChange::Copied(String::from("x.rs")),
                NameChange::Renamed {
                    from: String::from("src/a b.rs"),
                    to: String::from("src/b.rs"),
                },
                NameChange::Changed(String::from("src/a b.rs")),
                NameChange::Added(String::from("src/a b.rs")),
            ]
        );
        assert!(parse_name_changes(b"R100\0only one\0").is_err());
    }

    #[test]
    fn stops_at_the_first_addition_or_copy() {
        let renamed = [
            NameChange::Changed(String::from("b.rs")),
            NameChange::Renamed {
                from: String::from("a.rs"),
                to: String::from("b.rs"),
            },
            NameChange::Added(String::from("a.rs")),
            NameChange::Changed(String::from("older.rs")),
        ];
        assert_eq!(names("b.rs", &renamed), ["b.rs", "a.rs"]);
    }
}
