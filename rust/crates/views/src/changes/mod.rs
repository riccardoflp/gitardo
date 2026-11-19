//! The Changes page (`changes.pane`): the commit box at the top, with its message, **Commit** or **Amend**, **Amend last
//! commit** and the identity under it; then the files in conflict apart, and the one list of the changed files, each
//! with its box; a file opened from it shows on the right of the list, which stays as a column (`changes.beside-file`).

mod actions;
#[cfg(test)]
mod actions_tests;
mod beside;
mod co_authors;
mod commit_box;
mod menu;
mod options;
mod rows;
mod section;
mod skipped;

use std::collections::BTreeSet;

use model::changes::{ChangesList, OpenOnClick};
use model::details::TreeNode;
use model::state::{FileChange, Identity};
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Ui};

pub use section::{section_count, section_height, section_view};

use crate::details::{FileLayout, FilesIntent};
use crate::intent::Intent;
use crate::repo::{NoRepositoryState, no_repository_view};
use crate::text::changes as words;

pub use actions::{RowAction, buttons_kept, conflict_actions, file_actions, heading_actions, staying};
pub use beside::{BesideFile, BesideFileState, BesideShown, beside_file_view, beside_request};
pub use co_authors::{AUTHORS_SHOWN, CoAuthorsIntent, CoAuthorsState, co_authors_view};
pub use commit_box::{CommitBox, CommitKind, Draft};
pub use menu::{RowMenuState, row_menu};
pub use options::{CommitOptionsState, OptionsIntent, commit_options_view};

/// What the Changes page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangesState<'a> {
    /// No repository is shown: none in the workspace, or the one shown is being read.
    NoRepository(NoRepositoryState),
    /// The changes of the repository shown.
    Repository(Box<ChangesRepository<'a>>),
}

/// The changes of the repository shown, as the page shows them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangesRepository<'a> {
    /// The one list of the changed files, and the files in conflict.
    pub list: &'a ChangesList,
    /// Each row of the list as a file, in the same order, for its name and the tree.
    pub files: &'a [FileChange],
    /// The tree of the files, for the tree layout.
    pub tree: &'a [TreeNode],
    /// How the files are laid out.
    pub layout: FileLayout,
    /// The directories of the tree folded, by their paths.
    pub folded: &'a BTreeSet<String>,
    /// The commit box.
    pub commit: CommitBox<'a>,
    /// The identity commits are made with.
    pub identity: &'a Identity,
    /// The options of the next commit (`commit.options`).
    pub options: Box<CommitOptionsState>,
    /// Whether an operation runs: the buttons that change something are disabled.
    pub is_running: bool,
    /// Whether the keyboard goes to the message box in this frame, as **Show changes** in Quick search asks.
    pub wants_focus: bool,
    /// `changes.openOnClick`: what a click on a file does.
    pub on_click: OpenOnClick,
    /// The banner of an operation that stopped, over the lists (`conflicts.banner`).
    pub banner: Option<crate::conflicts::BannerState<'a>>,
    /// The banner of another worktree shown in this window, at the top (`worktree.banner`).
    pub worktree: Option<Box<crate::worktree::BannerState<'a>>>,
    /// The file shown beside the list, which its row marks; none while nothing is shown beside it, or every file
    /// (`changes.beside-file`).
    pub marked: Option<&'a str>,
    /// A number that changes each time another file is shown beside the list, which brings its row into sight once.
    pub serial: u64,
    /// The files hidden from git, listed under the changes (`changes.skipped-list`).
    pub skipped: &'a [model::state::SkippedFile],
    /// Whether the group of the files hidden from git is folded.
    pub is_hidden_folded: bool,
}

/// Something asked of the Changes page, or of the draft of the timeline, which shows its message and **Commit**.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangesIntent {
    /// The message typed in the commit box.
    Type(String),
    /// **Amend last commit** ticked or unticked.
    Amend(bool),
    /// **Commit**, **Amend** or `Ctrl+Enter` in the message box: `commit.create`, or `commit.amend` while **Amend last
    /// commit** is ticked.
    Commit,
    /// Something asked of the list of the files: its layout, or a directory folded.
    Files(FilesIntent),
    /// **Review** in the draft, or **Show changes** in Quick search: the page, with the keyboard in its message box.
    Open,
    /// The keyboard went to the message box.
    Focused,
    /// The sparkle: draft the message from the changes with the AI provider (`commit.draft-message`).
    DraftMessage,
    /// Stop drafting the message: the box is left as it was.
    StopDraft,
    /// Something asked of the options of the next commit (`commit.options`).
    Options(OptionsIntent),
    /// Something asked of the picker of the co-authors (`commit.co-authors`).
    CoAuthors(CoAuthorsIntent),
    /// A file of the list, or every file, opened on the right of the list (`changes.beside-file`).
    OpenBeside(BesideFile),
    /// `Alt+Down` or `Alt+Up` beside the list: the next or the previous file of the list, in the same place.
    StepFile(crate::diff::FileStep),
    /// **Changes** in the trail, or the Changes view of the bar of views: the whole page again, nothing beside it.
    ShowWhole,
    /// The heading of the files hidden from git: fold the group, or unfold it (`changes.skipped-list`).
    FoldHidden,
}

/// The Changes page, over the rectangle its caller clipped it to: with no repository, what the timeline says in its
/// place; otherwise the banner of an operation that stopped, the commit box, the identity, **Conflicts** and
/// **Changes**.
///
/// Feature: changes.pane
pub fn pane(state: &ChangesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let repository = match state {
        ChangesState::NoRepository(state) => return no_repository_view(state, ui),
        ChangesState::Repository(repository) => repository,
    };
    let spacing = ui.theme().sizes.spacing;
    let insets = Insets::symmetric(spacing.huge, spacing.large);
    page_body(repository, ui, (insets, Part::Page))
}

/// Where the parts of the Changes page are drawn: in the page, whole or as its column beside a file, or in the
/// **Changes** section of the side column (`changes.section`), which the screen reader names apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    /// The Changes page.
    Page,
    /// The **Changes** section of the side column.
    Section,
}

/// The parts of the page over the rectangle its caller clipped it to, within insets: wide on the page alone, narrow in
/// the column beside a file (`changes.beside-file`) and in the section of the side column.
fn page_body(repository: &ChangesRepository<'_>, ui: &mut Ui<'_>, (insets, part): (Insets, Part)) -> Vec<Intent> {
    let rect = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let inner = inset(rect, insets);
    let mut intents = Vec::new();
    let named = match part {
        Part::Page => (rect, AccessRole::Main, words::PAGE),
        Part::Section => (rect, AccessRole::Region, words::SECTION),
    };
    ui.region("changes page", named, |ui| {
        let mut area = Area::with_gap(inner, spacing.large);
        if let Some(banner) = &repository.worktree {
            let top = area.top(crate::filter::bar_height(ui));
            intents.extend(ui.clip(top, |ui| crate::worktree::banner(banner, ui)));
        }
        if let Some(banner) = &repository.banner {
            let top = area.top(crate::filter::bar_height(ui));
            intents.extend(ui.clip(top, |ui| crate::conflicts::banner_view(banner, ui)));
        }
        let top = area.top(commit_box::height(ui));
        intents.extend(commit_box::view(repository, ui, top));
        intents.extend(rows::view(repository, ui, area.rest()));
    });
    intents
}

/// The height the parts of the page take within insets, with every row of its lists in sight: what `page_body` lays
/// out, for the section of the side column, which is no taller than what it shows where it starts.
fn body_height(repository: &ChangesRepository<'_>, ui: &Ui<'_>, insets: Insets) -> f32 {
    let sizes = ui.theme().sizes;
    let banner = crate::filter::bar_height(ui) + sizes.spacing.large;
    let banners = [repository.worktree.is_some(), repository.banner.is_some()];
    let above: f32 = banners.into_iter().filter(|is_shown| *is_shown).map(|_| banner).sum();
    let rows = crate::number::float_of_count(rows::row_count(repository)) * sizes.row_height;
    insets.top + above + commit_box::height(ui) + sizes.spacing.large + rows + insets.bottom
}
