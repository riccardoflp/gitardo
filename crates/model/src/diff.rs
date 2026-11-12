//! The two versions of a file the Diff page compares.

/// The text of each version of a file, as bytes; none for a version that is not there: a file not yet in the index or
/// in `HEAD`, deleted on disk, or the side of a commit with no parent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DiffSides {
    /// The version on the left.
    pub left: Option<Vec<u8>>,
    /// The version on the right.
    pub right: Option<Vec<u8>>,
}
