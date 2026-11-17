//! The pages, opened in place of the timeline and the inspector with a trail above them, `Timeline › Welcome`, whose
//! first step comes back to the timeline.

use ui::Ui;
use ui::layout::Area;

use crate::commit_page::{CommitPageId, CommitPageState, view as commit_page_view};
use crate::compare::{ComparePageId, CompareState, page as compare_page_view};
use crate::diff::{DiffPageId, DiffPageState, page_view as diff_page_view};
use crate::file_view::{FilePageId, FilePageState, page_view as file_page_view};
use crate::graph::ReferencesShown;
use crate::intent::Intent;
use crate::line::is_narrow;
use crate::settings::{SettingsPageState, settings_view};
use crate::text::pages as words;
use crate::text::settings as settings_words;
use crate::welcome::{WelcomeState, page_view};

mod trail;

/// A page of the window. Each is open once: asking for it again shows the one there is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Page {
    /// The Welcome page (`welcome.page`).
    Welcome,
    /// The page of a commit (`commit-page.view`).
    Commit(CommitPageId),
    /// The settings page (`settings.edit`).
    Settings,
    /// The Changes page (`changes.pane`).
    Changes,
    /// A Diff page (`diff.page`).
    Diff(DiffPageId),
    /// The interactive rebase page (`rebase.todo-page`).
    Rebase,
    /// A comparison page (`compare.page`).
    Compare(ComparePageId),
    /// A Merge page (`merge-view.page`).
    Merge(crate::merge_view::MergePageId),
    /// The review of the resolutions of the conflicts (`review.page`).
    Review,
    /// A File page (`file-view.page`).
    File(FilePageId),
    /// The Files page: the tree of the working tree.
    Files,
    /// The page that chooses what new worktrees get (`worktree-setup.page`).
    NewWorktrees,
}

impl Page {
    /// Its name in the trail.
    #[must_use]
    pub fn title(self) -> &'static str {
        match self {
            Page::Welcome => words::WELCOME,
            Page::Commit(_) => crate::text::details::PAGE,
            Page::Settings => settings_words::PAGE,
            Page::Changes => crate::text::changes::PAGE,
            Page::Diff(_) => crate::text::diff::PAGE,
            Page::Rebase => crate::text::rebase::PAGE,
            Page::Compare(_) => crate::text::compare::PAGE,
            Page::Merge(_) => crate::text::merge_view::PAGE,
            Page::Review => crate::text::review::PAGE,
            Page::File(_) => crate::text::file_view::PAGE,
            Page::Files => crate::text::files::PAGE,
            Page::NewWorktrees => crate::text::worktree_setup::PAGE,
        }
    }
}

/// What a page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageState<'a> {
    /// The Welcome page.
    Welcome(WelcomeState<'a>),
    /// The page of a commit.
    Commit(CommitPageState<'a>),
    /// The settings page.
    Settings(SettingsPageState<'a>),
    /// The Changes page.
    Changes(crate::changes::ChangesState<'a>),
    /// The Changes page with a file opened beside its list, the page as a column on the left
    /// (`changes.beside-file`).
    ChangesBeside(Box<crate::changes::BesideFileState<'a>>),
    /// A Diff page.
    Diff(DiffPageState<'a>),
    /// The interactive rebase page.
    Rebase(crate::rebase::TodoPageState<'a>),
    /// A comparison page.
    Compare(CompareState<'a>),
    /// A comparison page with a file opened, its diff on the left and the comparison on the right
    /// (`compare.beside-diff`).
    CompareBeside(Box<crate::compare::BesideDiffState<'a>>),
    /// A Merge page, with its title.
    Merge(crate::merge_view::MergePageState<'a>, &'a str),
    /// The review of the resolutions of the conflicts.
    Review(crate::review::ReviewState<'a>),
    /// A File page.
    File(Box<FilePageState<'a>>),
    /// The Files page.
    Files(crate::files::FilesPageState<'a>),
    /// The Files page with a file opened beside its tree (`file-view.beside-files`).
    FilesBeside(Box<crate::file_view::BesideFilesState<'a>>),
    /// The page that chooses what new worktrees get, with its title.
    NewWorktrees(crate::worktree_setup::SetupPageState<'a>, &'a str),
}

impl<'a> PageState<'a> {
    /// Which page it is.
    #[must_use]
    pub fn page(&self) -> Page {
        match self {
            PageState::Welcome(_) => Page::Welcome,
            PageState::Commit(commit) => Page::Commit(commit.id),
            PageState::Settings(_) => Page::Settings,
            PageState::Changes(_) | PageState::ChangesBeside(_) => Page::Changes,
            PageState::Diff(diff) => Page::Diff(diff.id),
            PageState::Rebase(_) => Page::Rebase,
            PageState::Compare(compare) => Page::Compare(compare.id),
            PageState::CompareBeside(beside) => Page::Compare(beside.compare.id),
            PageState::Merge(merge, _) => Page::Merge(merge.id),
            PageState::Review(_) => Page::Review,
            PageState::File(file) => Page::File(file.id),
            PageState::Files(_) | PageState::FilesBeside(_) => Page::Files,
            PageState::NewWorktrees(..) => Page::NewWorktrees,
        }
    }

    /// Its name in the trail: the name of the page, or the short hash and the subject of a commit.
    #[must_use]
    pub fn title(&self) -> &'a str {
        match self {
            PageState::Commit(commit) => commit.title,
            PageState::Welcome(_)
            | PageState::Settings(_)
            | PageState::Changes(_)
            | PageState::Rebase(_)
            | PageState::Files(_) => self.page().title(),
            PageState::Diff(diff) => diff.title,
            PageState::Compare(compare) => compare.title,
            PageState::CompareBeside(beside) => beside.diff.title,
            PageState::Merge(_, title) | PageState::NewWorktrees(_, title) => title,
            PageState::Review(review) => review.title,
            PageState::File(file) => file.title,
            PageState::FilesBeside(beside) => beside.file.title,
            PageState::ChangesBeside(beside) => beside.title(),
        }
    }

    /// The step of the trail between the pages it was reached through and the name of the page, with what it asks:
    /// **Files**, which shows the tree alone again, while a file is opened beside it; **Changes**, which shows the
    /// whole page again, while a file is opened beside its list; the comparison, which shows it whole again, while the
    /// diff of one of its files is shown beside it.
    #[must_use]
    pub fn step(&self) -> Option<(&'a str, Intent)> {
        match self {
            PageState::CompareBeside(beside) => Some((
                beside.compare.title,
                Intent::Compare(crate::compare::CompareIntent::ShowWhole(beside.compare.id)),
            )),
            PageState::FilesBeside(_) => Some((
                crate::text::files::PAGE,
                Intent::Files(crate::files::FilesPageIntent::ShowTree),
            )),
            PageState::ChangesBeside(_) => Some((
                crate::text::changes::PAGE,
                Intent::Changes(crate::changes::ChangesIntent::ShowWhole),
            )),
            // No other page shows another inside it.
            _ => None,
        }
    }

    /// Whether it puts the side column away while it is shown, and so has no button for it in its trail: a comparison
    /// with the diff of one of its files beside it (`compare.beside-diff`).
    #[must_use]
    pub fn puts_side_away(&self) -> bool {
        matches!(self, PageState::CompareBeside(_))
    }

    /// Whether it shows a file with edits not saved, which puts a dot after its name in the trail (`file-view.edit`).
    #[must_use]
    pub fn is_unsaved(&self) -> bool {
        match self {
            PageState::File(file) => file.is_unsaved,
            PageState::Diff(diff) => diff.is_unsaved,
            PageState::FilesBeside(beside) => beside.file.is_unsaved,
            PageState::ChangesBeside(beside) => beside.is_unsaved(),
            PageState::CompareBeside(beside) => beside.diff.is_unsaved,
            _ => false,
        }
    }
}

/// The page shown, with its trail.
#[derive(Clone, Debug, PartialEq)]
pub struct PageHostState<'a> {
    /// Whether the trail is shown: not while the window shows no workspace, when there is no timeline to come back to.
    pub has_trail: bool,
    /// Where the side column is, for the button at the start of the trail that puts it away and brings it back
    /// (`repo.side-column`); none without a workspace, and on a page that puts the side column away itself.
    pub side: Option<crate::SidePlace>,
    /// The pages the page was reached through, after the timeline and before it, each with its name, the first
    /// opened first (`ui.page-trail`).
    pub way: Vec<(Page, &'a str)>,
    /// The page.
    pub page: PageState<'a>,
}

/// The page shown, over the rectangle its caller clipped it to, under its title row: the button that puts the side
/// column away and brings it back, at the same place as in the top row of the timeline, then the trail: **Timeline**,
/// which comes back to the timeline, the pages the page was reached through, each going back to it, and the name of
/// the page; and a button that closes it. `Alt+Left` goes back one name (`ui.page-trail`).
///
/// Feature cards that fill it: every page of the catalog opens here (`commit-page.view`, `compare.page`, the Diff,
/// Merge, Conflicts, File, Files, Changes, Settings and Worktree setup pages), each adding its own steps to the trail
/// (`Timeline › Commit 4b830a6 › src/login/page.rs`).
pub fn host_view(state: &PageHostState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let mut intents = Vec::new();
    let rect = ui.current_clip();
    let mut area = Area::new(rect);
    if state.has_trail {
        let bar = area.top(crate::graph::top_row_height(ui));
        let side = state
            .side
            .filter(|_| !state.page.puts_side_away())
            .map(|place| ReferencesShown::of(place, is_narrow(ui)));
        intents.extend(trail::trail_view(state, side, ui, bar));
    }
    let body = area.rest();
    intents.extend(ui.clip(body, |ui| match &state.page {
        PageState::Welcome(welcome) => page_view(welcome, ui),
        PageState::Commit(commit) => commit_page_view(commit, ui),
        PageState::Settings(settings) => settings_view(settings, ui),
        PageState::Changes(changes) => crate::changes::pane(changes, ui),
        PageState::ChangesBeside(beside) => crate::changes::beside_file_view(beside, ui),
        PageState::Diff(diff) => diff_page_view(diff, ui),
        PageState::Rebase(rebase) => crate::rebase::todo_page_view(rebase, ui),
        PageState::Compare(compare) => compare_page_view(compare, ui),
        PageState::CompareBeside(beside) => crate::compare::beside_diff_view(beside, ui),
        PageState::Merge(merge, _) => crate::merge_view::page_view(merge, ui),
        PageState::Review(review) => crate::review::page_view(review, ui),
        PageState::File(file) => file_page_view(file, ui),
        PageState::Files(files) => crate::files::page_view(files, ui),
        PageState::FilesBeside(beside) => crate::file_view::beside_files_view(beside, ui),
        PageState::NewWorktrees(setup, _) => crate::worktree_setup::page(setup, ui),
    }));
    // `Alt+Left` is read after the page, which may use it in a field.
    if state.has_trail && trail::is_back_pressed(ui) {
        intents.push(trail::back_one(state));
    }
    intents
}
