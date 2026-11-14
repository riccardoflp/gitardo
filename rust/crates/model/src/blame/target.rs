//! What the operations of the blame are given, what they ask git, and what they ask of the window after.

use super::BlameLine;
use crate::CommitId;

/// A file of a File page and the line of its cursor, for the operations of the blame (`blame.toggle-file`,
/// `blame.open-line-commit`, `blame.reveal-line-commit`, `blame.previous`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineOfFile {
    /// The path of the file from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version the page shows; none for the file of the working tree.
    pub revision: Option<CommitId>,
    /// The line of the cursor, from 1.
    pub line: usize,
    /// The text of the page when it has edits not saved, as it would be written.
    pub unsaved: Option<Vec<u8>>,
    /// The blame the page shows of the line of the cursor.
    pub shown: ShownBlame,
}

/// The blame a File page shows of the line of its cursor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShownBlame {
    /// The page shows no blame.
    Hidden,
    /// The page shows the blame of its file: who last changed the line, or none when git said nothing of it.
    Line(Option<BlameLine>),
}

/// A blame of a whole file to read (`blame.read-file`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameQuery {
    /// The file, from the top of the working tree, by the name it has in the revision when one is given.
    pub path: String,
    /// The commit to blame the file as of; none for the file as it is.
    pub revision: Option<CommitId>,
    /// The text of the File page when it has edits not saved.
    pub unsaved: Option<Vec<u8>>,
    /// Whether whitespace and moved lines are left to who wrote them (`blame.ignoreWhitespaceAndMoves`).
    pub ignores_whitespace: bool,
}

/// The blame of a file to show beside its lines, in the File page of the file at the revision (`blame.file`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameOfFile {
    /// The path of the file from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version the page shows; none for the file of the working tree.
    pub revision: Option<CommitId>,
    /// Who last changed each line, from the first.
    pub lines: Vec<Option<BlameLine>>,
}

/// A file to open in a File page, at a line, with its blame shown (`blame.previous`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileToOpen {
    /// The blame of the file, its path and its revision.
    pub blame: BlameOfFile,
    /// The line to open it at, from 1.
    pub line: usize,
}
