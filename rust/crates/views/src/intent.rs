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
    /// Show the repository of a tab, by its place in the island of tabs (`repo.switch-tab`).
    SwitchTab(usize),
    /// Shows in a tab, by its place, a repository of its group by its folder: the repository itself, one of its
    /// worktrees or one of its submodules, clicked among the rows under its row (`repo.group-tabs`).
    ShowInTab(usize, PathBuf),
    /// Folds or unfolds the rows of the worktrees and the submodules under the row of a tab, by its place
    /// (`repo.group-tabs`).
    FoldTab(usize),
    /// Opens a repository of the workspace in a tab, by its folder (`repo.open-tab`).
    OpenTab(PathBuf),
    /// **Open Repository in a Tab** of Quick search: the picker of the repositories with no tab (`repo.open-tab`).
    PickTabToOpen,
    /// The repositories checked in the picker of the tabs to open, by their places; none when it was cancelled
    /// (`repo.choose-tabs`).
    TabsChosen(Option<Vec<usize>>),
    /// Closes the tab at a place (`repo.close-tab`).
    CloseTab(usize),
    /// Closes every tab but the one at a place (`repo.close-tab`).
    CloseOtherTabs(usize),
    /// Closes the tab shown, from Quick search (`repo.close-tab`).
    CloseTabShown,
    /// Shows the folder of the tab at a place in the file manager of the system, from the menu of its row
    /// (`repo.tabs`).
    RevealTab(usize),
    /// **Open workspace file…**: the system's dialog for a workspace file of VS Code (`repo.open-code-workspace`).
    OpenWorkspaceFile,
    /// **Select repository…**: the picker of the repositories of the workspace (`repo.select`).
    PickRepository,
    /// The picker of the repositories closed with nothing chosen.
    ClosePicker,
    /// **Check again**: look for the repositories of the workspace again and read the first one
    /// (`repo.no-repository`).
    CheckAgain,
    /// **Refresh**: read the repository shown again at once.
    Refresh,
    /// **Read the repository**: the window opened with `readOnStartup` off reads it now (`repo.read-state`).
    ReadRepository,
    /// **Show the commands Gitardo runs**: the bottom panel shown at the Command log (`log.show`, `panel.bottom`).
    ShowCommandLog,
    /// Something asked of the bottom panel (`panel.bottom`).
    Panel(crate::panel::PanelIntent),
    /// Something asked of the terminals of the bottom panel (`terminal.view`).
    Terminal(crate::terminal::TerminalIntent),
    /// **Theme…**: the picker of the themes (`settings.theme`).
    PickTheme,
    /// The picker of the themes closed with nothing chosen.
    CloseThemePicker,
    /// A value of the settings page, or the theme picked, written in a settings file (`settings.edit`).
    SetSetting(crate::settings::SettingChange),
    /// The file the settings page writes changes in (`settings.edit`).
    ChooseSettingsFile(crate::settings::SettingsTarget),
    /// Something asked of the settings page itself: its list scrolled to a section (`settings.sections`).
    Settings(crate::settings::SettingsIntent),
    /// Show the timeline: the first step of a trail, or **Show the timeline** on the Welcome page.
    ShowTimeline,
    /// Show a page, opening it when it is not open; a page is open once.
    ShowPage(Page),
    /// `Ctrl+Tab`: show the next of the timeline and the pages open, in turn.
    NextPage,
    /// A name of the title row of a page clicked, or `Alt+Left`: back to that page, as it was left, dropping the pages
    /// after it from the way (`ui.page-trail`).
    BackTo(Page),
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
    /// Open an address in the browser of the system: a pull request on GitHub (`github.open-pull-request`).
    OpenUrl(String),
    /// Something asked of the timeline.
    Graph(GraphIntent),
    /// Something asked of the search, of **Go to commit…** or of Quick search.
    Search(crate::search::SearchIntent),
    /// Something asked of the inspector (`details.show`).
    Details(DetailsIntent),
    /// Something asked of the page of a commit (`commit-page.view`).
    CommitPage(CommitPageId, CommitPageIntent),
    /// Something asked of the Diff pages (`diff.page`).
    Diff(crate::diff::DiffIntent),
    /// Something asked of the File pages.
    FileView(crate::file_view::FileViewIntent),
    /// Something asked of a comparison page or of the comparison in the inspector (`compare.page`, `compare.in-place`).
    Compare(crate::compare::CompareIntent),
    /// An intent of the review of the conflicts.
    Review(crate::review::ReviewIntent),
    /// An operation to run, or what is answered to the one running.
    Feature(crate::feature::FeatureIntent),
    /// Something asked of the Changes page or of the commit box of the draft (`changes.pane`).
    Changes(crate::changes::ChangesIntent),
    /// Something asked of the interactive rebase page (`rebase.todo-page`).
    Rebase(crate::rebase::RebaseIntent),
    /// Something asked of a file in conflict (`conflicts.open-file`).
    Conflicts(crate::conflicts::ConflictsIntent),
    /// Something asked of the Merge pages (`merge-view.page`).
    MergeView(crate::merge_view::MergeIntent),
    /// Something asked of the Files page.
    Files(crate::files::FilesPageIntent),
    /// An operation of the blame or of the history run on a File page, or the hover of its blame (`blame.file`,
    /// `blame.current-line`).
    Blame(crate::blame::BlameIntent),
    /// Something asked of the page that chooses what new worktrees get (`worktree-setup.page`).
    WorktreeSetup(crate::worktree_setup::SetupIntent),
    /// Something asked of the Tasks of the Activity drawer (`worktree-setup.command-steps`).
    Tasks(crate::activity::TaskIntent),
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
#[derive(Clone, Debug, PartialEq)]
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
    /// The arrow of a stash, by its commit: list its files, or hide them (`stash.pane`).
    ToggleStash(model::CommitId),
    /// **Show Stashes**: the branch rail shown with the Stashes list open (`stash.pane`).
    ShowStashes,
    /// List these branches in the rail, whatever the timeline shows; none is everything (`refs.list-filter`).
    List(Option<BranchesShown>),
    /// Open the list of the filter of the rail, in a window of this size, in logical pixels; none when the size is not
    /// known yet, as from Quick search (`refs.list-filter`).
    OpenListFilter {
        /// The size of the window it opened in: a window resized closes it.
        window: Option<(f32, f32)>,
    },
    /// Close the list of the filter of the rail.
    CloseListFilter,
    /// What is typed in the box of the filter of the rail.
    TypeInListFilter(String),
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
    /// A diff refused because a side has a NUL byte (`diff.page`).
    DiffLooksBinary,
    /// A diff refused because it took longer than its time to compute (`diff.page`).
    DiffTooComplex,
    /// A Merge page refused because the file is not in conflict with a version on each side (`merge-view.page`).
    MergeNotInConflict,
}
