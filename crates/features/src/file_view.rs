//! What the File page changes or starts: its text written to the disk, and the file opened in another editor
//! (`docs/features/17-diff-and-merge-views.md`).

mod open_external;
mod save;

pub use open_external::open_external;
pub use save::{ON_DISK, OVERWRITE, save};
