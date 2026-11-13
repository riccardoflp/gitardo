//! What the Diff page changes: one change staged, unstaged or reverted from the buttons of its gutter, and the whole
//! file staged, unstaged or discarded from its title buttons (`docs/features/17-diff-and-merge-views.md`).

mod change;
mod discard_file;
mod revert_change;
mod stage_change;
mod stage_file;
mod unstage_change;
mod unstage_file;

#[cfg(test)]
mod testing;

pub use change::VERSIONS;
pub use discard_file::discard_file;
pub use revert_change::revert_change;
pub use stage_change::stage_change;
pub use stage_file::stage_file;
pub use unstage_change::unstage_change;
pub use unstage_file::unstage_file;
