//! Where a reference has been, as its reflog keeps it.

use crate::{CommitId, Timestamp};

/// A place a reference has been, newest first in a reflog: `HEAD@{3}`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReflogPosition {
    /// The commit the reference was at.
    pub commit: CommitId,
    /// What moved it there: `checkout: moving from main to feature`.
    pub subject: String,
    /// How git names the position: `HEAD@{3}`.
    pub selector: String,
    /// The date of the commit, as `%ct` gives it.
    pub time: Timestamp,
}
