//! What the File page changes or starts: its text written to the disk, and the file opened in another editor
//! (`docs/features/17-diff-and-merge-views.md`).

mod open_external;
mod save;

pub use open_external::open_external;
pub(crate) use open_external::{quoted, system_opener};
pub use save::{ON_DISK, OVERWRITE, save};

use std::path::{Path, PathBuf};

/// Where a file of the working tree is on disk, from the root of the system, in the separators of the system: git
/// gives the folder of a repository with `/` on Windows too, and the log and the commands show the path as the system
/// writes it.
pub(crate) fn on_disk(folder: &Path, path: &str) -> PathBuf {
    let mut absolute = folder.to_path_buf();
    absolute.extend(path.split('/').filter(|part| !part.is_empty()));
    if cfg!(windows) {
        PathBuf::from(absolute.display().to_string().replace('/', "\\"))
    } else {
        absolute
    }
}
