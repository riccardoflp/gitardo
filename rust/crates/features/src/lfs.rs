//! The files a repository stores with Git LFS (`docs/features/21-large-files.md`, D84): downloading them, setting up
//! Git LFS, storing a pattern with it and no longer, the files committed before a pattern, and the locks.

mod download;
mod lock;
mod set_up;
mod store_committed;
mod track;
mod unlock;
mod untrack;

pub use download::download;
pub use lock::lock;
pub use set_up::set_up;
pub use store_committed::store_committed;
pub use track::track;
pub use unlock::{UNCOMMITTED, unlock, warning as unlock_warning};
pub use untrack::untrack;

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

#[cfg(test)]
pub(crate) mod testing {
    use model::lfs::{LfsPattern, LfsStatus};
    use model::state::RepositoryState;

    /// A repository with Git LFS 3.7.1, set up or not, with patterns as `(pattern, file)`.
    pub(crate) fn lfs_state(is_set_up: bool, patterns: &[(&str, &str)]) -> RepositoryState {
        RepositoryState {
            lfs: LfsStatus {
                version: Some(String::from("3.7.1")),
                is_set_up,
                patterns: patterns
                    .iter()
                    .enumerate()
                    .map(|(index, (pattern, file))| LfsPattern {
                        pattern: (*pattern).to_owned(),
                        file: (*file).to_owned(),
                        line: index + 1,
                    })
                    .collect(),
            },
            ..crate::testing::state()
        }
    }

    /// The same repository without Git LFS.
    pub(crate) fn without_lfs() -> RepositoryState {
        RepositoryState {
            lfs: LfsStatus::default(),
            ..lfs_state(true, &[("*.psd", ".gitattributes")])
        }
    }
}
