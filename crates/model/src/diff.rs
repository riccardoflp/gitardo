//! The Diff page of one file, as data: the two versions it compares, their lines put in rows level with each other
//! (`diff.align`), the unchanged rows folded away (`diff.collapse-unchanged`), how far one side scrolls when the other
//! does (`diff.scroll-together`), which changes are staged already (`diff.staged-changes`), and the order in which the
//! changed files follow each other (`diff.navigate`).

mod align;
mod change;
mod files;
mod fold;
mod scroll;
mod staged;

#[cfg(test)]
mod tests;

pub use align::{
    AlignInput, AlignedDiff, AlignedRow, ChangeRows, LineChange, LineChangeKind, RowLine, WordMarks, align,
};
pub use change::{DiffChange, DiffSide, FileVersions};
pub use files::{compare_paths, sorted_paths};
pub use fold::{FOLD_KEEPS, FOLD_LEAST, ShownRow, shown_rows, unchanged_folds};
pub use scroll::{ScrollInput, scroll_together};
pub use staged::{StagedInput, staged_changes};

/// The text of each version of a file, as bytes; none for a version that is not there: a file not yet in the index or
/// in `HEAD`, deleted on disk, or the side of a commit with no parent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffSides {
    /// The version on the left.
    pub left: Option<Vec<u8>>,
    /// The version on the right.
    pub right: Option<Vec<u8>>,
}

impl DiffSides {
    /// Whether either version has a NUL byte, which makes the file look binary.
    #[must_use]
    pub fn looks_binary(&self) -> bool {
        [&self.left, &self.right]
            .into_iter()
            .flatten()
            .any(|bytes| bytes.contains(&0))
    }
}
