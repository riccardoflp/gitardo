//! What the Merge page changes: the file saved and staged, which tells git its conflicts are resolved
//! (`docs/features/17-diff-and-merge-views.md`).

mod mark_resolved;

pub use mark_resolved::{MARK_ANYWAY, mark_resolved};
