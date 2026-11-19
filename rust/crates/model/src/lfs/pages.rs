//! A file stored with Git LFS as the pages show it (`lfs.show-in-pages`): the sides whose content is not on this
//! computer, what **Download** is asked to download (`lfs.download`), and the diff git gives of two pointers, said as
//! the sizes of their contents.

use crate::CommitId;
use crate::commit_page::{FilePatch, LineKind};

use super::pointer;

/// A file stored with Git LFS whose content is not on this computer, shown in place of its text with **Download**.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredWithLfs {
    /// The file, from the top of the working tree, with `/`.
    pub path: String,
    /// The commit whose version is shown, which **Download** fetches into the store; none for the commit checked out,
    /// whose files **Download** pulls into the working tree.
    pub commit: Option<CommitId>,
    /// The versions of the file that are pointers: one in the File page, one or two in the Diff page, in the order
    /// of the sides.
    pub sides: Vec<StoredSide>,
}

impl StoredWithLfs {
    /// What **Download** downloads: this file, of its commit.
    #[must_use]
    pub fn download(&self) -> LfsDownload {
        LfsDownload {
            paths: vec![LfsPath {
                path: self.path.clone(),
                is_folder: false,
            }],
            commit: self.commit.clone(),
        }
    }
}

/// A version of a file that is a pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSide {
    /// Which side of the page it is.
    pub side: SideOfPage,
    /// The size of its content, in bytes.
    pub size: u64,
    /// The id of its content, 64 hexadecimal digits.
    pub oid: String,
    /// Whether its content is in the store of the repository.
    pub is_downloaded: bool,
}

/// Where a version of a file is shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SideOfPage {
    /// The only version, of the File page.
    Only,
    /// The left side of the Diff page, the version before.
    Before,
    /// The right side of the Diff page, the version after.
    After,
}

/// What **Download** downloads (`lfs.download`): some files or folders, or every file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LfsDownload {
    /// The files and the folders; empty for every file stored with Git LFS.
    pub paths: Vec<LfsPath>,
    /// The commit whose files are fetched into the store, the working tree left as it is; none for the commit
    /// checked out, whose files are pulled into the working tree.
    pub commit: Option<CommitId>,
}

/// A file or a folder to download, from the top of the working tree, with `/`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LfsPath {
    /// Its path.
    pub path: String,
    /// Whether it is a folder, named to Git LFS as `<folder>/**`.
    pub is_folder: bool,
}

/// The sizes of the contents of a file stored with Git LFS whose diff is that of its pointers: none on the side where
/// the file is not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointerChange {
    /// The size before, in bytes.
    pub before: Option<u64>,
    /// The size after, in bytes.
    pub after: Option<u64>,
}

/// The sizes of the two contents when the lines of a diff are those of pointers of Git LFS, the lines `oid` and `size`
/// removed and added: the version before is its lines kept and removed, the version after its lines kept and added,
/// and each is a pointer or nothing. None for any other diff, or a diff whose hunks were left out.
#[must_use]
pub fn pointer_change(patch: &FilePatch) -> Option<PointerChange> {
    if patch.is_binary || patch.hunks.is_empty() {
        return None;
    }
    let version = |kept: LineKind| {
        let mut text = String::new();
        for line in patch.hunks.iter().flat_map(|hunk| &hunk.lines) {
            if line.kind == LineKind::Context || line.kind == kept {
                text.push_str(&line.text);
                text.push('\n');
            }
        }
        text
    };
    let side = |text: String| -> Result<Option<u64>, ()> {
        if text.is_empty() {
            return Ok(None);
        }
        pointer(text.as_bytes()).map(|found| Some(found.size)).ok_or(())
    };
    let before = side(version(LineKind::Removed)).ok()?;
    let after = side(version(LineKind::Added)).ok()?;
    (before.is_some() || after.is_some()).then_some(PointerChange { before, after })
}
