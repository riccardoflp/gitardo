//! The page of a commit: what a commit, or a comparison, changed in each of its files, as lines, and which of the diffs
//! start folded.

mod folds;
mod lines;
mod names;

use crate::state::{ChangeKind, FileChange};

pub use folds::{FILE_FOLD_LIMIT, Fold, FoldSet, PAGE_FOLD_LIMIT, initial_folds};
pub use lines::diff_lines;

/// How many lines a page is given at most, hunk headers included, for all of its files together.
pub const LINE_LIMIT: usize = 20_000;

/// What git printed for a diff, with what it is split by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiffOutput<'a> {
    /// The files listed for the same diff, in the order git lists them, as `git diff --name-status` gives them.
    pub files: &'a [FileChange],
    /// What `git diff` printed, with `a/` and `b/` before the names.
    pub output: &'a [u8],
    /// How many lines the files may have together, hunk headers included: [`LINE_LIMIT`].
    pub limit: usize,
}

/// The lines of each file of a diff, or why they cannot be shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatchLines {
    /// The lines of each file listed, in the order listed.
    Files(Vec<FilePatch>),
    /// The output of git names other files than those listed, or is not a diff this program reads: the lines are not
    /// shown, rather than shown against the wrong files.
    Unmatched,
}

/// What a diff changed in one file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilePatch {
    /// Its path, after the change.
    pub path: String,
    /// Its path before a rename or a copy.
    pub original_path: Option<String>,
    /// How it changed.
    pub kind: ChangeKind,
    /// Its hunks; none for a binary file, a file whose text did not change, or a file past the limit of lines.
    pub hunks: Vec<Hunk>,
    /// How many lines it gained, also when its hunks are left out.
    pub added: usize,
    /// How many lines it lost, also when its hunks are left out.
    pub removed: usize,
    /// Whether git said it is binary.
    pub is_binary: bool,
    /// Whether its hunks were left out because the files before it took the lines a page is given.
    pub is_past_limit: bool,
}

impl FilePatch {
    /// How many lines its diff has: the lines of its hunks, and one for each hunk header.
    #[must_use]
    pub fn size(&self) -> usize {
        self.hunks.iter().map(|hunk| hunk.lines.len() + 1).sum()
    }
}

/// A run of lines that changed, with the lines around them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hunk {
    /// Its header as git printed it: `@@ -12,7 +12,8 @@ fn parse`.
    pub header: String,
    /// Its lines.
    pub lines: Vec<DiffLine>,
}

/// A line of a hunk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    /// Whether it was kept, added or removed.
    pub kind: LineKind,
    /// Its text, without the mark git puts before it and without the line break.
    pub text: String,
    /// Its number in the version before; none for a line added and for the note on a missing newline.
    pub before: Option<usize>,
    /// Its number in the version after; none for a line removed and for the note on a missing newline.
    pub after: Option<usize>,
}

/// What a line of a hunk is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineKind {
    /// A line the two versions share.
    Context,
    /// A line only the version after has.
    Added,
    /// A line only the version before has.
    Removed,
    /// `\ No newline at end of file`, about the line before it.
    NoNewline,
}
