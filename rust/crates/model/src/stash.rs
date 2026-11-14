//! What a stash holds, what applying it would stop on, where it is shown in the timeline, and the stash a pull leaves.

mod autostash;
mod in_graph;
mod preview;

pub use autostash::{Notice, PullOutcome, autostash_left};
pub use in_graph::{GraphWithStashes, in_graph};
pub use preview::{StashPreview, StashPreviewInput, preview};

/// The files a stash changes, and the untracked files it took along, for what applying it would stop on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashFiles {
    /// The files it changes against the commit it was made on, from the top of the working tree.
    pub changed: Vec<String>,
    /// The untracked files it took along; none for a stash made without them.
    pub untracked: Vec<String>,
}

/// What is read of a stash before it is applied or popped: its files, those of its untracked files that are there
/// again, and the files a merge of it onto `HEAD` would leave in conflict.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashReading {
    /// The files it changes and the untracked files it took (`stash.read-files`).
    pub files: StashFiles,
    /// The untracked files it took that exist in the working tree.
    pub untracked_there: Vec<String>,
    /// The files `git merge-tree` names in conflict; none when git cannot tell.
    pub conflicts: Option<Vec<String>>,
}
