//! What the timeline is narrowed to by the history of a file, of some lines or of a folder, and what the operations
//! that narrow it are given and ask.

use std::path::PathBuf;

use super::LineRange;
use crate::CommitId;

/// The history the timeline is narrowed to, until **Show all commits** (`file-history.file`, `file-history.lines`,
/// `file-history.folder`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HistoryOf {
    /// The commits that changed a file, under every name it had.
    File {
        /// Its name now, from the top of the working tree.
        path: String,
        /// The names it had before, newest first.
        earlier: Vec<String>,
    },
    /// The commits that changed some lines of a file, followed back from a commit (`git log -L`).
    Lines {
        /// The name of the file in that commit, from the top of the working tree.
        path: String,
        /// The lines, as that commit has them.
        lines: LineRange,
        /// The commit the lines are followed back from: the one checked out when they were asked for.
        commit: CommitId,
        /// The lines selected in the editor, when they are not the lines of the commit.
        in_editor: Option<LineRange>,
    },
    /// The commits that changed anything in a folder.
    Folder {
        /// The folder, from the top of the working tree.
        path: String,
    },
}

impl HistoryOf {
    /// The names whose changes are listed, the name now first; the lines of a file name it alone.
    #[must_use]
    pub fn paths(&self) -> Vec<&str> {
        match self {
            HistoryOf::File { path, earlier } => std::iter::once(path.as_str())
                .chain(earlier.iter().map(String::as_str))
                .collect(),
            HistoryOf::Lines { path, .. } | HistoryOf::Folder { path } => vec![path.as_str()],
        }
    }

    /// The commit whose history alone is listed, in place of the branches the timeline shows: the commit the lines
    /// are followed back from.
    #[must_use]
    pub fn alone(&self) -> Option<&CommitId> {
        match self {
            HistoryOf::Lines { commit, .. } => Some(commit),
            HistoryOf::File { .. } | HistoryOf::Folder { .. } => None,
        }
    }
}

/// Some lines of the file of a File page, selected or under the cursor (`file-history.lines`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinesOfFile {
    /// The file on disk, by its whole path.
    pub file: PathBuf,
    /// The text of the page, saved or not.
    pub text: String,
    /// The lines of that text.
    pub lines: LineRange,
}

/// Some lines of a file as the File page has it, to find in the commit checked out
/// (`file-history.read-committed-lines`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommittedLinesQuery {
    /// The file, from the top of the working tree.
    pub path: String,
    /// The name the file had before, when the index has it as a rename: the name the commit has.
    pub renamed_from: Option<String>,
    /// The text of the file as the File page has it, saved or not.
    pub text: String,
    /// The lines of that text.
    pub lines: LineRange,
}

/// The lines of a text a selection stands for, from 1, both ends included: with no selection, the line of the cursor;
/// a selection that ends at the start of a line below the one it starts on leaves that line out; the empty line after
/// the last line break is the last line of the text. The selection goes from `start` to `end`, each a line from 0 and
/// a column in bytes, `start` first.
#[must_use]
pub fn selected_lines(text: &str, (start, end): ((usize, usize), (usize, usize))) -> LineRange {
    let count = super::lines(text).len().max(1);
    let mut last = end.0;
    if end.0 > start.0 && end.1 == 0 {
        last -= 1;
    }
    let first = start.0.min(count - 1) + 1;
    let last = last.min(count - 1) + 1;
    LineRange {
        start: first,
        end: last.max(first),
    }
}

#[cfg(test)]
#[allow(non_snake_case, reason = "tests are named <feature>__<what they check>")]
mod tests {
    use super::{LineRange, selected_lines};

    #[test]
    fn file_history_lines__selecting_from_line_4_to_the_start_of_line_10_asks_for_lines_4_9() {
        let text = (1..=12)
            .map(|line| format!("line {line}\n"))
            .collect::<Vec<_>>()
            .concat();
        assert_eq!(selected_lines(&text, ((3, 2), (9, 0))), LineRange { start: 4, end: 9 });
        assert_eq!(selected_lines(&text, ((3, 0), (8, 6))), LineRange { start: 4, end: 9 });
        assert_eq!(selected_lines(&text, ((5, 1), (5, 1))), LineRange { start: 6, end: 6 });
        assert_eq!(
            selected_lines(&text, ((12, 0), (12, 0))),
            LineRange { start: 12, end: 12 }
        );
    }
}
