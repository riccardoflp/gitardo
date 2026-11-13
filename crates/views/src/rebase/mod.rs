//! Rebasing and rewriting commits in the window: the entries of the menu of a commit and of a branch, the banner of a
//! rebase stopped to edit or split a commit (`rebase.edit-stop`), and the interactive rebase page (`rebase.todo-page`).

mod edit_stop;
mod menu;
#[cfg(test)]
mod tests;
mod todo_page;

pub use edit_stop::{EditStopState, edit_stop_view};
pub use menu::can_squash;
pub(crate) use menu::{RewriteMenu, rebase_onto_entry, rewrite_menu};
pub use todo_page::{TodoPage, TodoPageRow, TodoPageState, todo_page_view};

use model::CommitId;
use model::rebase::TodoAction;

/// What is asked of the interactive rebase page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RebaseIntent {
    /// **Interactive rebase of N commits…** on a commit: open the page with the commits from it up to `HEAD`.
    Open(CommitId),
    /// The action of the row at a place, newest first.
    SetAction(usize, TodoAction),
    /// A row moved from a place to another, by a drag or **Move up** and **Move down**.
    Move {
        /// Where it was.
        from: usize,
        /// Where it goes.
        to: usize,
    },
    /// The new subject typed for the row at a place.
    TypedSubject(usize, String),
    /// **Cancel**: close the page and run nothing.
    Cancel,
    /// **Start rebase**: close the page and run `rebase.interactive` with the rows, oldest first.
    Start,
}
