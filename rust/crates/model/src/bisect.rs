//! A bisect under way: the commits marked so far, and how many can still be the first bad one.

use crate::CommitId;

/// A bisect under way, as git keeps it.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Bisect {
    /// The commit marked bad, if one is.
    pub bad: Option<CommitId>,
    /// The commits marked good.
    pub good: Vec<CommitId>,
    /// The commits skipped, which cannot be tested.
    pub skipped: Vec<CommitId>,
    /// How many commits can still be the first bad one: those the bad commit reaches and no good commit reaches, the
    /// bad one included. None until a bad commit and a good one are marked.
    pub remaining: Option<u32>,
}
