//! The files a repository stores with Git LFS (`docs/features/21-large-files.md`, D84): downloading them, setting up
//! Git LFS, storing a pattern with it and no longer, the files committed before a pattern, and the locks.

mod download;

pub use download::download;

use model::contract::Refusal;

/// Said when Git LFS is not installed.
pub(crate) const NOT_INSTALLED: &str = "Git LFS is not installed: install it from git-lfs.com, then try again.";

/// The refusal of an operation of Git LFS when the program is not installed (`lfs.read-status`); none when it is.
pub(crate) fn not_installed(context: &model::contract::Context) -> Option<Refusal> {
    context
        .repository
        .lfs
        .version
        .is_none()
        .then(|| Refusal::saying(NOT_INSTALLED))
}
