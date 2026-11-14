//! Undo (`docs/features/16-undo-preview-log.md`).

mod journal;
mod last;

pub use last::{JOURNAL, last};
