//! The Changes page (`changes.pane`): the commit box at the top, with its message, **Commit** or **Amend**, **Amend last
//! commit** and the identity under it; then the files in conflict apart, and the one list of the changed files, each
//! with its box.

mod commit_box;
mod rows;

use std::collections::BTreeSet;

use model::changes::{ChangesList, OpenOnClick};
use model::details::TreeNode;
use model::state::{FileChange, Identity};
use ui::layout::{Area, Insets, inset};
use ui::{AccessRole, Ui};

use crate::details::{FileLayout, FilesIntent};
use crate::intent::Intent;
use crate::repo::{NoRepositoryState, no_repository_view};
use crate::text::changes as words;

pub use commit_box::{CommitBox, CommitKind};

/// What the Changes page shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangesState<'a> {
    /// No repository is shown: none in the workspace, or the one shown is being read.
    NoRepository(NoRepositoryState),
    /// The changes of the repository shown.
    Repository(ChangesRepository<'a>),
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
    /// Whether an operation runs: the buttons that change something are disabled.
    pub is_running: bool,
    /// Whether the keyboard goes to the message box in this frame, as **Show changes** in Quick search asks.
    pub wants_focus: bool,
    /// `changes.openOnClick`: what a click on a file does.
    pub on_click: OpenOnClick,
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
}

/// The Changes page, over the rectangle its caller clipped it to: with no repository, what the timeline says in its
/// place; otherwise the commit box, the identity, **Conflicts** and **Changes**.
///
/// Feature: changes.pane
pub fn pane(state: &ChangesState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let repository = match state {
        ChangesState::NoRepository(state) => return no_repository_view(state, ui),
        ChangesState::Repository(repository) => repository,
    };
    let rect = ui.current_clip();
    let spacing = ui.theme().sizes.spacing;
    let inner = inset(rect, Insets::symmetric(spacing.huge, spacing.large));
    let mut intents = Vec::new();
    ui.region("changes page", (rect, AccessRole::Main, words::PAGE), |ui| {
        let mut area = Area::with_gap(inner, spacing.large);
        let top = area.top(commit_box::height(ui));
        intents.extend(commit_box::view(repository, ui, top));
        intents.extend(rows::view(repository, ui, area.rest()));
    });
    intents
}
