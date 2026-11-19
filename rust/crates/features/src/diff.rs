//! What the Diff page changes: one change staged, unstaged or reverted from the buttons of its gutter, the lines selected staged,
//! unstaged or reverted from its context menu, and the whole
//! file staged, unstaged or discarded from its title buttons (`docs/features/17-diff-and-merge-views.md`).

mod change;
mod discard_file;
mod lines;
mod revert_change;
mod revert_lines;
mod stage_change;
mod stage_file;
mod stage_lines;
mod unstage_change;
mod unstage_file;
mod unstage_lines;

#[cfg(test)]
mod testing;

pub use change::VERSIONS;
pub use discard_file::discard_file;
pub use revert_change::{revert_change, reverted};
pub use revert_lines::{revert_lines, reverted_lines};
pub use stage_change::stage_change;
pub use stage_file::stage_file;
pub use stage_lines::stage_lines;
pub use unstage_change::unstage_change;
pub use unstage_file::unstage_file;
pub use unstage_lines::unstage_lines;
