//! What the operations of the conflicts are given of the files in conflict: which versions the index keeps of a file
//! (`conflicts.read-files`), and the texts the rules resolve (`conflicts.resolve-by-rules`).

/// A file in conflict, and which of the three versions git keeps in the index it has.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStages {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Whether it has stage 1, the common ancestor: not when both sides added the file.
    pub has_base: bool,
    /// Whether it has stage 2, the current side: not when that side deleted the file.
    pub has_current: bool,
    /// Whether it has stage 3, the incoming side: not when that side deleted the file.
    pub has_incoming: bool,
}

impl FileStages {
    /// Whether git still has both sides of it, so that `git checkout --merge` can mark its conflicts again.
    #[must_use]
    pub fn has_both_sides(&self) -> bool {
        self.has_current && self.has_incoming
    }
}

/// A file in conflict as it is on disk, and the three versions of it git keeps in the index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictTexts {
    /// Its path from the top of the working tree.
    pub path: String,
    /// Its bytes on disk; none when it is not there or is not a regular file.
    pub file: Option<Vec<u8>>,
    /// The base, `git show :1:<file>`; none when there is none.
    pub base: Option<Vec<u8>>,
    /// The current side, `git show :2:<file>`; none when it deleted the file.
    pub current: Option<Vec<u8>>,
    /// The incoming side, `git show :3:<file>`; none when it deleted the file.
    pub incoming: Option<Vec<u8>>,
}

/// The most files in conflict the rules look into: the others are counted as left.
pub const MOST_FILES_LOOKED_INTO: usize = 200;

/// How many conflicts the files in conflict have, and how many of them need no choice (`conflicts.resolve-by-rules`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct RuleCounts {
    /// The conflicts git marked in the files looked into.
    pub conflicts: usize,
    /// Those the rules resolve.
    pub resolvable: usize,
}
