//! What is not committed: the index, the working tree and the files in conflict.

/// What is not committed in a worktree.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct WorkingTree {
    /// The changes staged in the index.
    pub staged: Vec<FileChange>,
    /// The changes in the working tree that are not staged. A file changed in both is in both lists.
    pub unstaged: Vec<FileChange>,
    /// The files git does not track and does not ignore, every file and not their folders, from the top of the
    /// working tree.
    pub untracked: Vec<String>,
    /// The files in conflict. A file in conflict is in no other list.
    pub conflicted: Vec<ConflictedFile>,
}

impl WorkingTree {
    /// How many files are not committed, each counted once.
    #[must_use]
    pub fn file_count(&self) -> usize {
        let mut paths: Vec<&str> = self
            .staged
            .iter()
            .chain(&self.unstaged)
            .map(|change| change.path.as_str())
            .collect();
        paths.extend(self.untracked.iter().map(String::as_str));
        paths.extend(self.conflicted.iter().map(|file| file.path.as_str()));
        paths.sort_unstable();
        paths.dedup();
        paths.len()
    }

    /// Whether nothing is changed, staged, untracked or in conflict.
    #[must_use]
    pub fn is_clean(&self) -> bool {
        self.staged.is_empty() && self.unstaged.is_empty() && self.untracked.is_empty() && self.conflicted.is_empty()
    }
}

/// A file changed in the index or in the working tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileChange {
    /// Its path from the top of the working tree, with `/`.
    pub path: String,
    /// Its path before a rename or a copy.
    pub original_path: Option<String>,
    /// What changed.
    pub kind: ChangeKind,
}

/// How a file changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChangeKind {
    /// It is new.
    Added,
    /// Its content or its mode changed.
    Modified,
    /// It is gone.
    Deleted,
    /// It was renamed, and maybe changed.
    Renamed,
    /// It was copied from another file.
    Copied,
    /// It became a link, a submodule or a file, from another of these.
    TypeChanged,
}

/// A file in conflict.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictedFile {
    /// Its path from the top of the working tree.
    pub path: String,
    /// What each side did to it.
    pub kind: ConflictKind,
}

/// What each side of a conflict did to a file, as `git status` says it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictKind {
    /// `DD`: deleted on both sides.
    BothDeleted,
    /// `AU`: added by us.
    AddedByUs,
    /// `UD`: deleted by them, changed by us.
    DeletedByThem,
    /// `UA`: added by them.
    AddedByThem,
    /// `DU`: deleted by us, changed by them.
    DeletedByUs,
    /// `AA`: added on both sides.
    BothAdded,
    /// `UU`: changed on both sides.
    BothModified,
}
