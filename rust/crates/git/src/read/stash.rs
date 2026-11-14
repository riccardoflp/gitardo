//! The stashes: the list of them, and what one changes.

mod files;
mod list;

pub use files::{StashRef, read_files, stash_changes_arguments, stash_untracked_arguments};
pub use list::{StashListRequest, read_list};
