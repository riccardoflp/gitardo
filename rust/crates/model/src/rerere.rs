//! Git's rerere, which records how conflicts are resolved and writes the same resolution the next time it meets them
//! (D84): whether it is on for a repository, and whether it also stages what it writes.

use std::path::PathBuf;

/// Whether git remembers resolutions in a repository (`rerere.read-state`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RerereState {
    /// Whether git records resolutions and writes them again: `rerere.enabled`, or, while it is not set, a folder
    /// `rr-cache` in the git folder.
    pub is_on: bool,
    /// Whether git stages what it writes from a recorded resolution: `rerere.autoUpdate`.
    pub stages: bool,
    /// Whether the configuration sets `rerere.autoUpdate`, to `true` or `false`.
    pub sets_stages: bool,
}

/// What `rerere.read-state` needs: the git folder of the worktree, `.git` or `.git/worktrees/<name>`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RerereStateRequest {
    /// The git folder of the worktree read.
    pub git_folder: PathBuf,
}
