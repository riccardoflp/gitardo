//! The tracked files git is told not to look at (`changes.read-skipped`).

/// A tracked file git is told not to look at, and how.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SkippedFile {
    /// Its path from the top of the working tree, as the index has it.
    pub path: String,
    /// How git is told to leave it alone.
    pub how: Skipped,
}

/// How git is told to leave a file alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Skipped {
    /// Skipped in the working tree (`--skip-worktree`): its changes stay out of commits, and a checkout leaves it alone.
    Worktree,
    /// Assumed unchanged (`--assume-unchanged`): a promise, for speed, that git may forget.
    AssumedUnchanged,
    /// Both.
    Both,
}

impl Skipped {
    /// Whether the file is marked skip-worktree.
    #[must_use]
    pub fn is_skip_worktree(self) -> bool {
        matches!(self, Skipped::Worktree | Skipped::Both)
    }

    /// Whether the file is marked assume-unchanged.
    #[must_use]
    pub fn is_assumed_unchanged(self) -> bool {
        matches!(self, Skipped::AssumedUnchanged | Skipped::Both)
    }
}
