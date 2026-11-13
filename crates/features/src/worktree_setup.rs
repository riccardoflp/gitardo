//! What new worktrees get of what git ignores, and the commands they run: the settings saved from the page and from
//! the Files page, the setup of a new worktree, and what worktrees that are there already are given.

mod bring;
pub mod files;
mod give;
mod new_worktree;
mod remove;
mod save;

pub use bring::{bring_into_other_worktrees, bring_into_worktree};
pub use give::{CHOOSE_ON_THE_PAGE, copy_into_new_worktrees, link_into_new_worktrees, stop_copying, stop_linking};
pub use new_worktree::{NEW_WORKTREE_TAKE, NewWorktree, new_worktree_steps};
pub use remove::remove_from;
pub use save::save;
