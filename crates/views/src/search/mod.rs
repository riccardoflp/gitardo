//! The search: the box at the top of the timeline (`search.box`) with what it finds among the loaded commits or in the
//! whole history (`search.history`), the count and the steps through the matches, a hash gone to wherever it is
//! (`search.go-to-hash`), the commits found shown as a list (`search.list-mode`); **Go to commit…**
//! (`search.go-to-commit`); and Quick search in the header (`Ctrl+K`).

mod go_to_commit;
mod go_to_hash;
mod history;
mod list_mode;
mod quick;
mod search_box;
#[cfg(test)]
mod tests;

use model::search::{BoxMode, Direction};

pub use go_to_commit::{GoToCommitDialog, GoToCommitVerdict, go_to_commit_dialog};
pub use go_to_hash::go_to_hash_view;
pub use history::{HistorySearch, MatchCount, history_view};
pub use list_mode::{ListMode, Narrowed, list_mode_view};
pub use quick::{QuickChoice, QuickCommand, QuickEntry, QuickSearchState, quick_search_view};
pub use search_box::{SearchBox, box_view, mode_entries};

/// Something asked of the search, of **Go to commit…** or of Quick search.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SearchIntent {
    /// A mode chosen in the list before the box.
    Mode(BoxMode),
    /// What is typed in the box.
    Type(String),
    /// `Enter` in the box: the search of the whole history asked, a step to the next match, or the hash gone to.
    Submit,
    /// A step to the next or the previous match (`search.step`).
    Step(Direction),
    /// `Escape` in the box: it is emptied.
    Empty,
    /// The funnel pressed (`search.list-mode`).
    ToggleList,
    /// **Show all commits** in the bar of the list.
    ShowAllCommits,
    /// **Go to commit…** asked (`search.go-to-commit`).
    GoToCommit,
    /// What is typed in **Go to commit**.
    GoToCommitTyped(String),
    /// **Go to commit** pressed, or `Enter` in its box.
    GoToCommitSubmit,
    /// **Go to commit** closed.
    GoToCommitClose,
    /// Quick search opened, with what was typed in its box in the header.
    OpenQuick(String),
    /// An entry of Quick search chosen.
    ChooseQuick(QuickChoice),
    /// Quick search closed with nothing chosen.
    CloseQuick,
}
