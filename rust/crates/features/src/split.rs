//! Splitting a commit: its changes taken out of it into the working tree, to commit them again in as many commits as
//! wanted, and going on once they are (`docs/features/07-history-rewriting.md`).

mod commit;
mod continue_split;

pub use commit::commit;
pub use continue_split::{FILES, LEFT, MESSAGE, continue_split};

/// The file, in `rebase-merge`, that says which commit a stopped rebase is splitting; it goes with the rest of that
/// folder when the rebase ends.
pub const SPLIT_FILE: &str = "gitardo-split";
