//! What a Diff page is made of: the two versions it compares, the files, and each file's two sides in rows.

use std::ops::Range;

use model::CommitId;
use model::diff::{AlignedDiff, LineChange};

use crate::text::diff as words;

/// A version of a file a side of the page shows.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum DiffVersion {
    /// What is checked out, followed when it moves.
    Head,
    /// The index.
    Index,
    /// The working tree.
    WorkingTree,
    /// A commit, by the name its column has: the name of a branch, or its first 7 hexadecimal digits.
    Commit {
        /// The commit.
        commit: CommitId,
        /// Its name over the column.
        name: String,
    },
    /// No version: every line of the other side is added, or removed.
    Empty,
}

impl DiffVersion {
    /// A commit named by its first 7 hexadecimal digits.
    #[must_use]
    pub fn commit(commit: CommitId) -> DiffVersion {
        DiffVersion::Commit {
            name: commit.short().to_owned(),
            commit,
        }
    }

    /// Its name over its column.
    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            DiffVersion::Head => words::HEAD,
            DiffVersion::Index => words::INDEX,
            DiffVersion::WorkingTree => words::WORKING_TREE,
            DiffVersion::Commit { name, .. } => name,
            DiffVersion::Empty => words::EMPTY,
        }
    }

    /// Whether it changes without a commit being made: `HEAD`, the index or the working tree, which the page reads
    /// again when the repository changes.
    #[must_use]
    pub fn is_live(&self) -> bool {
        matches!(self, DiffVersion::Head | DiffVersion::Index | DiffVersion::WorkingTree)
    }
}

/// A file of the page, by its path on each side, which differ for a renamed file.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiffFile {
    /// Its path on the left, from the top of the repository, with `/`.
    pub left: String,
    /// Its path on the right.
    pub right: String,
}

impl DiffFile {
    /// A file of the same path on both sides.
    #[must_use]
    pub fn same(path: &str) -> DiffFile {
        DiffFile {
            left: path.to_owned(),
            right: path.to_owned(),
        }
    }

    /// Its name, the last part of its path on the right.
    #[must_use]
    pub fn name(&self) -> &str {
        self.right.rsplit('/').next().unwrap_or(&self.right)
    }

    /// Its path as its header says it: `old/path → new/path` for a renamed file.
    #[must_use]
    pub fn shown_path(&self) -> String {
        if self.left == self.right {
            self.right.clone()
        } else {
            words::renamed(&self.left, &self.right)
        }
    }
}

/// What the user asks to see in a Diff page: two versions and the files compared between them, one after the other.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct DiffTarget {
    /// The version on the left.
    pub left: DiffVersion,
    /// The version on the right.
    pub right: DiffVersion,
    /// The files, in order.
    pub files: Vec<DiffFile>,
    /// The title of a page of several files, such as the short hash and subject of their commit; a page of one file is
    /// titled **\<file name\> (\<left\> ↔ \<right\>)**.
    pub title: Option<String>,
}

impl DiffTarget {
    /// The diff of one file between two versions.
    #[must_use]
    pub fn file(left: DiffVersion, right: DiffVersion, file: DiffFile) -> DiffTarget {
        DiffTarget {
            left,
            right,
            files: vec![file],
            title: None,
        }
    }

    /// The title of its page.
    #[must_use]
    pub fn page_title(&self) -> String {
        match (&self.title, self.files.as_slice()) {
            (Some(title), _) => title.clone(),
            (None, [file]) => words::title(file.name(), self.left.name(), self.right.name()),
            (None, _) => words::title("", self.left.name(), self.right.name()),
        }
    }
}

/// How far the diff of a file is.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub enum FileDiff {
    /// It is read and computed.
    #[default]
    Loading,
    /// A side has a NUL byte.
    Binary,
    /// It took longer than its time to compute.
    TooComplex,
    /// Git could not read a side.
    Failed,
    /// Its two sides in rows.
    Text(Box<DiffText>),
}

/// The two sides of a file and their lines in rows.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct DiffText {
    /// The text of the left side, as UTF-8 with what is not replaced.
    pub left: String,
    /// The text of the right side.
    pub right: String,
    /// Whether each side is there: a side that is not is named **Empty**.
    pub sides: (bool, bool),
    /// Where each line of the left side is in its text.
    pub left_lines: Vec<Range<usize>>,
    /// Where each line of the right side is in its text.
    pub right_lines: Vec<Range<usize>>,
    /// The changes, by the lines each takes on each side, in order.
    pub changes: Vec<LineChange>,
    /// The rows.
    pub aligned: AlignedDiff,
    /// For each change, whether it is in the index already (`diff.staged-changes`); empty when that says nothing.
    pub staged: Vec<bool>,
    /// The runs of unchanged rows that fold (`diff.collapse-unchanged`).
    pub folds: Vec<Range<usize>>,
}

impl DiffText {
    /// A line of the left side.
    #[must_use]
    pub fn left_line(&self, line: usize) -> &str {
        text_of(&self.left, &self.left_lines, line)
    }

    /// A line of the right side.
    #[must_use]
    pub fn right_line(&self, line: usize) -> &str {
        text_of(&self.right, &self.right_lines, line)
    }

    /// Whether a change is in the index already.
    #[must_use]
    pub fn is_staged(&self, change: usize) -> bool {
        self.staged.get(change).copied().unwrap_or(false)
    }
}

/// The text of a line of a side.
fn text_of<'a>(text: &'a str, lines: &[Range<usize>], line: usize) -> &'a str {
    lines
        .get(line)
        .and_then(|range| text.get(range.clone()))
        .unwrap_or_default()
}

/// A row of the page, as its list draws it.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum PageLine {
    /// The header of a file, by its place: its path.
    Header(usize),
    /// A row of the diff of a file.
    Row(usize, usize),
    /// The bar of a fold of a file: the rows it hides.
    Folded(usize, Range<usize>),
    /// What a file with no rows says: being read, binary, too complex, or no change.
    Note(usize),
}

/// Where a change is: its file, its place among the changes of the file, and the row of the page it starts at.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ChangePlace {
    /// The file.
    pub file: usize,
    /// The change, among those of the file.
    pub change: usize,
    /// The row of the page of its first row.
    pub line: usize,
}

/// What the bar of the page says for a moment after `F7` or `Shift+F7`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Hint {
    /// *Press F7 again to go to the next file*.
    NextFile,
    /// *Press Shift+F7 again to go to the previous file*.
    PreviousFile,
    /// *No other changed files.*
    NoOtherFiles,
}

impl Hint {
    /// Its words.
    #[must_use]
    pub fn words(self) -> &'static str {
        match self {
            Hint::NextFile => words::NEXT_FILE_HINT,
            Hint::PreviousFile => words::PREVIOUS_FILE_HINT,
            Hint::NoOtherFiles => words::NO_OTHER_FILES,
        }
    }
}
