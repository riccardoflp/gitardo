//! Who last changed the lines of a file: one line, for the line under the cursor, or every line, beside the file.

mod groups;

use std::ffi::OsString;
use std::path::PathBuf;

use model::blame::BlameLine;

use crate::read::fields::arguments;
use crate::{Git, GitError, Output};

use groups::parse_groups;

/// Whether the blame leaves a line to who wrote it when all that was changed since is its whitespace or its place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Whitespace {
    /// Every change counts.
    #[default]
    Counted,
    /// Changes of whitespace and lines moved do not count: `-w -M` (`blame.ignoreWhitespaceAndMoves`).
    IgnoredWithMoves,
}

/// The line of a file to blame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLineRequest {
    /// The file, from the folder git runs in.
    pub path: PathBuf,
    /// The line, from 1.
    pub line: usize,
    /// Whether whitespace and moves count.
    pub whitespace: Whitespace,
    /// The text of the file as the File page has it, when it has edits not saved; none to blame the file on disk.
    pub contents: Option<Vec<u8>>,
}

/// Who last changed one line of a file: its commit, author, date and summary, or a line not committed yet. When git
/// fails, as for an untracked file or a line past the end of the committed file, there is no blame for the line.
///
/// Feature: blame.read-line
///
/// # Errors
///
/// When git cannot be started, or prints what this does not understand.
pub fn read_line(git: &Git, request: &BlameLineRequest) -> Result<Option<BlameLine>, GitError> {
    let output = run(git, &line_arguments(request), request.contents.as_deref())?;
    if !output.is_success() {
        // Git fails for a file it does not track or a line it does not have: the line has no blame, which is not an
        // error the user needs to see.
        return Ok(None);
    }
    parse_line(&output.stdout)
}

/// `git blame --porcelain -L <n>,<n> [-w -M] [--contents -] -- <file>`.
#[must_use]
pub fn line_arguments(request: &BlameLineRequest) -> Vec<OsString> {
    let range = format!("{0},{0}", request.line);
    let mut words = arguments(&["blame", "--porcelain", "-L", &range]);
    words.extend(options(request.whitespace, request.contents.as_deref()));
    words.push(OsString::from("--"));
    words.push(request.path.clone().into_os_string());
    words
}

/// Who last changed the line that [`line_arguments`] blamed.
///
/// # Errors
///
/// When a group or a detail cannot be read.
pub fn parse_line(output: &[u8]) -> Result<Option<BlameLine>, GitError> {
    let blame = parse_groups(output)?;
    Ok(blame.groups.first().and_then(|group| blame.line_of(group)))
}

/// The file to blame, and as of when.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameFileRequest {
    /// The file, from the folder git runs in, by the name it has in the revision when one is given.
    pub path: PathBuf,
    /// Whether whitespace and moves count.
    pub whitespace: Whitespace,
    /// The text of the file as the File page has it, when it has edits not saved.
    pub contents: Option<Vec<u8>>,
    /// The commit to blame the file as of, for a file as it was in a commit; none for the file as it is.
    pub revision: Option<String>,
}

/// Who last changed each line of a file, from the first line: a commit, a line not committed yet, or nothing for a
/// line git said nothing of. When git fails, the file is in no commit and there is no blame.
///
/// Feature: blame.read-file
///
/// # Errors
///
/// When git cannot be started, or prints what this does not understand.
pub fn read_file(git: &Git, request: &BlameFileRequest) -> Result<Option<Vec<Option<BlameLine>>>, GitError> {
    let output = run(git, &file_arguments(request), request.contents.as_deref())?;
    if !output.is_success() {
        // Git fails for a file it does not track: the file has no blame, which the File page says in its own words.
        return Ok(None);
    }
    parse_file(&output.stdout).map(Some)
}

/// `git blame --incremental [-w -M] [--contents -] [<revision>] -- <file>`.
#[must_use]
pub fn file_arguments(request: &BlameFileRequest) -> Vec<OsString> {
    let mut words = arguments(&["blame", "--incremental"]);
    words.extend(options(request.whitespace, request.contents.as_deref()));
    words.extend(request.revision.iter().map(OsString::from));
    words.push(OsString::from("--"));
    words.push(request.path.clone().into_os_string());
    words
}

/// Who last changed each line that [`file_arguments`] blamed, from the first line to the last git named.
///
/// # Errors
///
/// When a group or a detail cannot be read.
pub fn parse_file(output: &[u8]) -> Result<Vec<Option<BlameLine>>, GitError> {
    let blame = parse_groups(output)?;
    let line_count = blame
        .groups
        .iter()
        .map(|group| group.first_line.saturating_add(group.count).saturating_sub(1))
        .max()
        .unwrap_or(0);
    let mut lines = vec![None; line_count];
    for group in &blame.groups {
        let line = blame.line_of(group);
        let first = group.first_line.saturating_sub(1);
        for slot in lines.iter_mut().skip(first).take(group.count) {
            slot.clone_from(&line);
        }
    }
    Ok(lines)
}

/// The options both reads share: `-w -M` when whitespace and moves do not count, and `--contents -` with the text on
/// standard input.
fn options(whitespace: Whitespace, contents: Option<&[u8]>) -> Vec<OsString> {
    let mut words = Vec::new();
    if whitespace == Whitespace::IgnoredWithMoves {
        words.extend(arguments(&["-w", "-M"]));
    }
    if contents.is_some() {
        words.extend(arguments(&["--contents", "-"]));
    }
    words
}

/// Runs a blame, with the text of the file on its standard input when there is one.
fn run(git: &Git, words: &[OsString], contents: Option<&[u8]>) -> Result<Output, GitError> {
    match contents {
        Some(contents) => git.read_output_with_input(words, contents),
        None => git.read_output(words),
    }
}
