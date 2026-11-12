//! What the user wants, in the words of the domain: what every view returns, and what the store of the window reduces.

use std::collections::BTreeSet;
use std::path::PathBuf;

use model::filter::{BranchesShown, ReferenceKind};
use model::refs::SortOrder;

use crate::commit_page::{CommitPageId, CommitPageIntent};
use crate::details::DetailsIntent;
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
    /// Something asked of the branch rail.
    Rail(RailIntent),
    /// Something asked of the branches the timeline shows.
    Filter(FilterIntent),
    /// Put the notice shown away.
    DismissNotice,
    /// Something asked of the timeline.
    Graph(GraphIntent),
    /// Something asked of the search, of **Go to commit…** or of Quick search.
    Search(crate::search::SearchIntent),
    /// Something asked of the inspector (`details.show`).
    Details(DetailsIntent),
    /// Something asked of the page of a commit (`commit-page.view`).
    CommitPage(CommitPageId, CommitPageIntent),
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

/// A list of the branch rail.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RailList {
    /// **Favorites** (`refs.favorite`).
    Favorites,
    /// **Recent** (`refs.recent`).
    Recent,
    /// The local branches.
    Local,
    /// The branches of a remote, by its name.
    Remote(String),
    /// The stashes, kept aside.
    Stashes,
    /// The worktrees, kept aside.
    Worktrees,
    /// The tags, kept aside.
    Tags,
    /// The submodules, kept aside.
    Submodules,
}

impl RailList {
    /// Whether the list is kept aside: those start closed, the others open.
    #[must_use]
    pub fn is_kept_aside(&self) -> bool {
        matches!(
            self,
            RailList::Stashes | RailList::Worktrees | RailList::Tags | RailList::Submodules
        )
    }
}

/// The lists of the branch rail opened or closed, and those listed in full, until the application is closed.
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct RailLists {
    /// The lists opened or closed against how they start: the lists kept aside start closed, the others open.
    pub turned: BTreeSet<RailList>,
    /// The lists of more than 100 entries listed in full, until they are closed.
    pub in_full: BTreeSet<RailList>,
}

impl RailLists {
    /// Whether a list is open.
    #[must_use]
    pub fn is_open(&self, list: &RailList) -> bool {
        list.is_kept_aside() == self.turned.contains(list)
    }

    /// Whether a list is listed in full.
    #[must_use]
    pub fn is_in_full(&self, list: &RailList) -> bool {
        self.in_full.contains(list)
    }
}

/// Something asked of the branch rail.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RailIntent {
    /// Open or close a list.
    Toggle(RailList),
    /// **Show all `<n>`…**: every entry of a list, until it is closed.
    InFull(RailList),
    /// What is typed in the filter box at the top of the branch rail (`refs.filter`).
    Type(String),
    /// **Add to favorites** or **Remove from favorites** on a branch or a remote branch, by its full name
    /// (`refs.favorite`).
    Favorite {
        /// The full name of the branch.
        reference: String,
        /// Whether it is a favorite after.
        is_favorite: bool,
    },
    /// **Sort by name** or **Sort by date** (`refs.sort`).
    Sort(SortOrder),
}

/// Something asked of the branches the timeline shows.
#[derive(Clone, Debug, PartialEq)]
pub enum FilterIntent {
    /// Show these branches; none is everything.
    Show(Option<BranchesShown>),
    /// A kind of reference switched in **…** (`filter.kinds`).
    SwitchKind(ReferenceKind),
    /// Open the list of the branch filter, in a window of this size, in logical pixels (`filter.picker`).
    OpenPicker {
        /// The size of the window it opened in: a window resized closes it.
        window: (f32, f32),
    },
    /// Close the list of the branch filter.
    ClosePicker,
    /// What is typed in the box of the branch filter.
    Type(String),
    /// Whether the branch rail lists only the branches the timeline shows: `branches.followGraphFilter`
    /// (`filter.list-too`).
    FollowGraphFilter(bool),
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
