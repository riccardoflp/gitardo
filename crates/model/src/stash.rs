//! What a stash holds.

/// The files a stash changes, and the untracked files it took along, for what applying it would stop on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StashFiles {
    /// The files it changes against the commit it was made on, from the top of the working tree.
    pub changed: Vec<String>,
    /// The untracked files it took along; none for a stash made without them.
    pub untracked: Vec<String>,
}
