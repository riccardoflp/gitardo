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

/// A file in conflict that git resolved by itself from a resolution it recorded (`rerere.read-resolved`).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedAsBefore {
    /// Its path from the top of the working tree, with `/`.
    pub path: String,
    /// Whether git staged it, as it does with `rerere.autoUpdate`: it is no longer in conflict.
    pub is_staged: bool,
}

/// What `rerere.read-resolved` needs: the git folder of the worktree and the files in conflict.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedRequest {
    /// The git folder of the worktree read, which holds `MERGE_RR`.
    pub git_folder: PathBuf,
    /// The files in conflict, from the top of the working tree.
    pub conflicted: Vec<String>,
}

/// What `rerere.read-marked` needs: the file, and the names of its two sides for its markers.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MarkedRequest {
    /// The path of the file from the top of the working tree.
    pub path: String,
    /// The name of the current side, after `<<<<<<<`.
    pub current: String,
    /// The name of the incoming side, after `>>>>>>>`.
    pub incoming: String,
}
