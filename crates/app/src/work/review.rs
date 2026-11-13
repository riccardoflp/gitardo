//! The read of the review of the conflicts on a worker (`review.page`): the files in conflict (`conflicts.read-files`),
//! the conflicts git marked in each (`conflicts.parse-markers`) with their fingerprints (`review.fingerprint`), what
//! the rules make of them (`conflicts.resolve-by-rules`), what each side changed of the base (`review.side-diff`), the
//! files one side deleted (`review.deleted-card`), and the names of the two sides (`conflicts.side-names`).

mod kept;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::time::{Duration, Instant};

use git::read::conflicts::{ConflictFile, ConflictFilesRequest, read_files};
use git::read::merge_view::{MergeSidesRequest, read_sides};
use git::read::review::{SideNamesRequest, read_side_names};
use git::{CancelToken, Git, GitError};
use model::conflicts::side_names;
use model::diff::LineChange;
use model::review::{
    ConflictStretch, FileLine, ReviewConflict, ReviewFile, ReviewRead, RuleCase, RuleProposal, SideDiff, SideDiffInput,
    Which, side_diff,
};
use text::conflicts::{
    Line, MarkedConflict, RuleConflict, RuleResolutions, RulesInput, fingerprint, parse_markers, resolve_by_rules,
    split_lines,
};
use text::diff::{LineDiffInput, WordDiffInput, line_diff, word_diff};

/// The most files in conflict looked into.
const MOST_FILES: usize = 200;
/// The largest file looked into: 2 MB.
const LARGEST_FILE: usize = 2 * 1024 * 1024;
/// The longest the rules may take over a file.
const TIME_LIMIT: Duration = Duration::from_secs(5);

/// Which files the review reads.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct ReviewRequest {
    /// Only these files, by their paths from the top of the working tree; none for every file in conflict.
    pub only: Option<Vec<String>>,
}

/// What a worker read for the review: the files and what each side of each conflict changed of the base.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReviewReadout {
    /// The files and the names of the sides.
    pub read: ReviewRead,
    /// For each file of `read.files`, for each of its conflicts, what the current and the incoming side changed.
    pub side_diffs: Vec<Vec<(SideDiff, SideDiff)>>,
}

/// Reads what the review shows of the files in conflict.
///
/// # Errors
///
/// What git said when a read failed.
pub fn read_review(folder: &Path, request: &ReviewRequest, cancel: &CancelToken) -> Result<ReviewReadout, String> {
    let git = Git::new(folder).with_cancel(cancel.clone());
    read_all(&git, request, cancel).map_err(|error| error.to_string())
}

/// Reads every file asked for, the first 200 of them.
fn read_all(git: &Git, request: &ReviewRequest, cancel: &CancelToken) -> Result<ReviewReadout, GitError> {
    let names = read_side_names(git, &SideNamesRequest)?;
    let files = read_files(git, &ConflictFilesRequest)?;
    let mut read = ReviewRead {
        operation: names.operation,
        sides: side_names(&names.input()),
        files: Vec::new(),
        deleted: Vec::new(),
        others: Vec::new(),
    };
    let mut side_diffs = Vec::new();
    let asked = files
        .files
        .into_iter()
        .filter(|file| request.only.as_ref().is_none_or(|only| only.contains(&file.path)));
    for (index, file) in asked.enumerate() {
        if index >= MOST_FILES {
            read.others.push(file.path);
            continue;
        }
        if let Some(deleted_by) = deleted_by(&file) {
            match kept::read(git, &file, deleted_by)? {
                Some(deleted) => read.deleted.push(deleted),
                None => read.others.push(file.path),
            }
            continue;
        }
        match read_file(git, &file.path, cancel)? {
            Some((file, diffs)) => {
                read.files.push(file);
                side_diffs.push(diffs);
            }
            None => read.others.push(file.path),
        }
    }
    Ok(ReviewReadout { read, side_diffs })
}

/// The side that deleted a file the other changed, when one did.
fn deleted_by(file: &ConflictFile) -> Option<Which> {
    match (&file.base, &file.current, &file.incoming) {
        (Some(_), Some(_), None) => Some(Which::Incoming),
        (Some(_), None, Some(_)) => Some(Which::Current),
        _ => None,
    }
}

/// A file as the review keeps it, with what each side of each of its conflicts changed.
type FileRead = (ReviewFile, Vec<(SideDiff, SideDiff)>);

/// A file with conflicts marked in it, with what each side of each changed; none for a file binary, very large, not
/// UTF-8 or without markers that pair up.
fn read_file(git: &Git, path: &str, cancel: &CancelToken) -> Result<Option<FileRead>, GitError> {
    let sides = read_sides(
        git,
        &MergeSidesRequest {
            file: path.to_owned(),
            unsaved: None,
        },
    )?;
    if sides.result.len() > LARGEST_FILE || sides.result.contains(&0) {
        return Ok(None);
    }
    let Ok(text) = std::str::from_utf8(&sides.result) else {
        return Ok(None);
    };
    let Some(marked) = parse_markers(text).filter(|marked| !marked.is_empty()) else {
        return Ok(None);
    };
    let started = Instant::now();
    let keep_going = || started.elapsed() < TIME_LIMIT && !cancel.is_cancelled();
    let resolutions = resolve_by_rules(&RulesInput {
        file: &sides.result,
        base: &sides.base,
        current: &sides.current,
        incoming: &sides.incoming,
        keep_going: &keep_going,
    });
    let file = review_file(path, text, &marked, &resolutions);
    let diffs = file
        .conflicts
        .iter()
        .map(|conflict| {
            (
                diff_of(&file, conflict, Which::Current),
                diff_of(&file, conflict, Which::Incoming),
            )
        })
        .collect();
    Ok(Some((file, diffs)))
}

/// A file as the review keeps it: its text, and each conflict with its sides and what the rules make of it.
#[must_use]
pub fn review_file(path: &str, text: &str, marked: &[MarkedConflict], resolutions: &RuleResolutions<'_>) -> ReviewFile {
    let lines = split_lines(text);
    let ruled: &[RuleConflict<'_>] = match resolutions {
        RuleResolutions::LookedInto(conflicts) => conflicts,
        RuleResolutions::LeftAlone(_) => &[],
    };
    let texts = |range: &std::ops::Range<usize>| -> Vec<String> {
        lines
            .get(range.clone())
            .unwrap_or_default()
            .iter()
            .map(|line| line.text.to_owned())
            .collect()
    };
    let conflicts = marked
        .iter()
        .enumerate()
        .map(|(index, conflict)| {
            let span: Vec<&str> = lines
                .get(conflict.span.clone())
                .unwrap_or_default()
                .iter()
                .map(|line| line.text)
                .collect();
            let ruled = ruled.get(index);
            ReviewConflict {
                index,
                fingerprint: fingerprint(&span).0,
                line: conflict.line,
                span: conflict.span.clone(),
                current: texts(&conflict.current),
                base: conflict.base.as_ref().map(texts),
                incoming: texts(&conflict.incoming),
                rule: ruled
                    .and_then(|ruled| ruled.resolution.as_ref())
                    .map(|resolution| RuleProposal {
                        case: case_of(resolution.case),
                        replaced: resolution.replaced.clone(),
                        lines: resolution.lines.iter().map(owned_line).collect(),
                    }),
                stretch: ruled
                    .and_then(|ruled| ruled.stretch.as_ref())
                    .map(|stretch| ConflictStretch {
                        file_lines: stretch.file_lines.clone(),
                        base: stretch.base.iter().map(|line| line.text.to_owned()).collect(),
                        current: stretch.current.iter().map(|line| line.text.to_owned()).collect(),
                        incoming: stretch.incoming.iter().map(|line| line.text.to_owned()).collect(),
                    }),
            }
        })
        .collect();
    ReviewFile {
        path: path.to_owned(),
        text: text.to_owned(),
        conflicts,
    }
}

/// A line of the rules, owned.
fn owned_line(line: &Line<'_>) -> FileLine {
    FileLine::new(line.text, line.ending)
}

/// The case of the rules, in the words of the review.
fn case_of(case: text::conflicts::RuleCase) -> RuleCase {
    match case {
        text::conflicts::RuleCase::SpacesOrLineEndings => RuleCase::SpacesOrLineEndings,
        text::conflicts::RuleCase::SameChange => RuleCase::SameChange,
        text::conflicts::RuleCase::OneSide => RuleCase::OneSide,
        text::conflicts::RuleCase::NextToEachOther => RuleCase::NextToEachOther,
    }
}

/// What a side of a conflict changed of the base, with the line diff and the word diff of the Diff page.
#[must_use]
pub fn diff_of(file: &ReviewFile, conflict: &ReviewConflict, side: Which) -> SideDiff {
    side_diff(&SideDiffInput {
        file,
        conflict,
        side,
        changes: &changes_between,
        words: &|left, right| word_diff(&WordDiffInput { left, right }),
    })
}

/// The changes between two lists of lines; none when they take too long.
pub(crate) fn changes_between(left: &[&str], right: &[&str]) -> Vec<LineChange> {
    let started = Instant::now();
    let keep_going = || started.elapsed() < TIME_LIMIT;
    line_diff(&LineDiffInput {
        left,
        right,
        keep_going: &keep_going,
    })
    .map(|changes| {
        changes
            .iter()
            .map(|change| LineChange {
                left: change.left.lines(),
                right: change.right.lines(),
            })
            .collect()
    })
    .unwrap_or_default()
}
