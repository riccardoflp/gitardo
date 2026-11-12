//! What the user wants, in the words of the domain: what every view returns, and what the store of the window reduces.

use std::path::PathBuf;

use crate::graph::GraphIntent;
use crate::page::Page;

/// Something the user asked for in a place or a page of the window.
#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    /// **Open folder…**: ask for a folder in the system's dialog (`repo.open-folder`).
    OpenFolder,
    /// The answer to the question asked when a folder is opened while the window shows a workspace
    /// (`repo.open-folder`).
    AnswerOpenFolder(FolderAnswer),
    /// A recent workspace, by its place in the list of the recent ones, the latest first (`repo.open-recent`).
    OpenRecent(usize),
    /// A repository of the workspace to show, by its folder (`repo.select`).
    SelectRepository(PathBuf),
    /// **Select repository…**: the picker of the repositories of the workspace (`repo.select`).
    PickRepository,
    /// The picker of the repositories closed with nothing chosen.
    ClosePicker,
    /// **Check again**: look for the repositories of the workspace again and read the first one
    /// (`repo.no-repository`).
    CheckAgain,
    /// **Refresh**: read the repository shown again at once.
    Refresh,
    /// Show the timeline: the first step of a trail, or **Show the timeline** on the Welcome page.
    ShowTimeline,
    /// Show a page, opening it when it is not open; a page is open once.
    ShowPage(Page),
    /// `Ctrl+Tab`: show the next of the timeline and the pages open, in turn.
    NextPage,
    /// Close the page shown, which shows the timeline or the page before it.
    ClosePage,
    /// Open the Activity drawer (`activity.what-is-running`).
    OpenActivity,
    /// Close the Activity drawer.
    CloseActivity,
    /// Open or close a list of **Kept aside** in the branch rail.
    ToggleKept(KeptList),
    /// Put the notice shown away.
    DismissNotice,
    /// Something asked of the timeline.
    Graph(GraphIntent),
}

/// What is done with a folder opened while the window shows a workspace.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FolderAnswer {
    /// **Replace the workspace**, the default.
    Replace,
    /// **Add to the workspace**.
    Add,
    /// Nothing: the question was cancelled.
    Cancel,
}

/// A list of **Kept aside** in the branch rail.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum KeptList {
    /// The stashes.
    Stashes,
    /// The worktrees.
    Worktrees,
    /// The tags.
    Tags,
}

impl KeptList {
    /// Every list, in the order of the rail.
    pub const ALL: [KeptList; 3] = [KeptList::Stashes, KeptList::Worktrees, KeptList::Tags];
}

/// A sentence the window says once, over what it shows, until it is put away.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Notice {
    /// A recent workspace whose folders are all gone was refused, and taken out of the list (`repo.open-recent`).
    RecentGone,
    /// **Select repository…** in a workspace of one repository (`repo.select`).
    SingleRepository,
    /// **Select repository…** in a workspace with no repository (`repo.select`).
    NoRepositoryToSelect,
}
