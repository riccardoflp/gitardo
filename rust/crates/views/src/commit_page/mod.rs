//! The page of a commit (`commit-page.view`): its whole message with its labels, who made it and when, its hash and
//! parents, the files it changed with the lines each gained and lost, and on demand the diff of each, folded by the
//! rules of `commit-page.initial-folds`; **Show in the timeline** (`commit-page.show-in-graph`).
//!
//! The page is a list of rows of one height, the height of a line of code, so that a diff of 20,000 lines draws only
//! the lines in sight, find in page reaches every line of the diffs unfolded, and a diff can be brought to the top.

mod diff;
mod head;
mod rows;

use std::collections::BTreeSet;

use model::CommitId;
use model::commit_page::{Fold, FoldSet, PatchLines};
use model::details::{CommitDetail, TreeNode};
use model::graph::Label;
use ui::layout::{Insets, inset};
use ui::{AccessRole, Button, List, ListSearch, Rect, Reveal, RevealPlace, SelectionMode, Steering, Ui};

use crate::details::{CappedList, FileLayout, FilesIntent, FilesOwner, FilesState, file_rows, say};
use crate::icons;
use crate::intent::Intent;
use crate::text::details as words;

/// The number of a page of a commit, unique in its window.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CommitPageId(pub u64);

/// What the page of a commit shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CommitPageState<'a> {
    /// Which page it is.
    pub id: CommitPageId,
    /// Its title in the trail: the short hash and the subject.
    pub title: &'a str,
    /// Whether its commit is a stash: only the commit it was made on is its parent, and it is not a merge.
    pub is_stash: bool,
    /// How far its commit is read.
    pub read: PageRead<'a>,
    /// Whether **Show the changes** was clicked.
    pub shows_changes: bool,
    /// Which diffs are folded.
    pub folds: &'a FoldSet,
    /// How the files are laid out.
    pub layout: FileLayout,
    /// The tree of the files shown.
    pub tree: &'a [TreeNode],
    /// The directories of the tree folded.
    pub folded: &'a BTreeSet<String>,
    /// Whether **Show all** was clicked.
    pub is_all_files_shown: bool,
    /// The labels of the commit, as the timeline of its repository draws them; none while the window shows another.
    pub labels: &'a [Label],
    /// The color of the line of the commit in the timeline.
    pub lane: usize,
    /// What to bring to the top of the page, once for each number.
    pub reveal: Option<(u64, PageTarget)>,
    /// A number that changes whenever what the page lists changes, for find in page.
    pub version: u64,
    /// The signature of its commit (`details.signature`); none for a stash, or before it is asked.
    pub signature: Option<crate::details::SignatureRead<'a>>,
}

/// How far the commit of a page is read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageRead<'a> {
    /// It is read.
    Loading,
    /// It is read.
    Read(&'a PageData),
    /// It is not in the repository any more.
    Missing,
    /// Git could not read it.
    Failed,
}

/// A commit as its page shows it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageData {
    /// What git said of it.
    pub detail: CommitDetail,
    /// The title of its message.
    pub title: String,
    /// The rest of its message, unwrapped.
    pub text: String,
    /// The lines it changed in each file.
    pub patch: PatchLines,
}

/// Where the page goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PageTarget {
    /// Its top.
    Top,
    /// The diff of a file, by its place among the files.
    Diff(usize),
}

/// Something asked of the page of a commit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitPageIntent {
    /// Something asked of its files.
    Files(FilesIntent),
    /// Open the page of a parent.
    OpenParent(CommitId),
    /// **Show in the timeline**.
    ShowInTimeline,
    /// **Show the changes**, or **Hide the changes**.
    ShowChanges(bool),
    /// Fold or unfold the diff of a file, by its place.
    Fold(usize, Fold),
    /// **Fold all**, or **Unfold all**.
    FoldAll(Fold),
    /// The eye of a file: show the changes, and go to that file's diff, unfolded.
    ShowFile(usize),
    /// **Back to the top**.
    BackToTop,
}

/// The page of a commit, over the rectangle its caller clipped it to.
///
/// Feature: commit-page.view
pub fn view(state: &CommitPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let padding = ui.theme().sizes.spacing.huge;
    let inner = inset(rect, Insets::symmetric(padding, 0.0));
    let mut intents = Vec::new();
    ui.region("commit page", (rect, AccessRole::Main, words::PAGE), |ui| {
        let said = match state.read {
            PageRead::Loading => words::LOADING,
            PageRead::Missing => words::NOT_IN_REPOSITORY,
            PageRead::Failed => words::COULD_NOT_READ,
            PageRead::Read(data) => {
                intents = page_rows(state, data, ui, inner);
                return;
            }
        };
        say(ui, inset(inner, Insets::symmetric(0.0, padding)), said);
    });
    let _ = ui.find_in_page(("find in commit page", state.id), rect);
    intents
}

/// **Show in the timeline**, at the start of a rectangle: it brings the timeline to the commit, in place of the page,
/// pointing the window back at the repository of the page first when it shows another.
///
/// Feature: commit-page.show-in-graph
pub fn show_in_graph(state: &CommitPageState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    let rect = ui.current_clip();
    let button = Button::outline(words::SHOW_IN_TIMELINE).with_icon(icons::TIMELINE);
    let width = ui.button_size(&button).0.min(rect.width);
    if ui
        .button(
            ("show in timeline", state.id),
            Rect::new(rect.x, rect.y, width, rect.height),
            &button,
        )
        .clicked
    {
        return vec![Intent::CommitPage(state.id, CommitPageIntent::ShowInTimeline)];
    }
    Vec::new()
}

/// The height of a row of the page: a line of code.
pub(crate) fn row_height(ui: &Ui<'_>) -> f32 {
    ui.line_height(ui.text_style(ui::TextKind::Code)) + 4.0
}

/// The rows of a page read, as a list that draws those in sight, with the header of the diff scrolled through kept at
/// the top and the button that goes back to the top once the page is scrolled down.
fn page_rows(state: &CommitPageState<'_>, data: &PageData, ui: &mut Ui<'_>, rect: Rect) -> Vec<Intent> {
    let files = FilesState {
        files: &data.detail.files,
        tree: state.tree,
        layout: state.layout,
        folded: state.folded,
        capped: CappedList {
            owner: FilesOwner::Page(state.id),
            count: data.detail.files.len(),
            is_all_shown: state.is_all_files_shown,
        },
    };
    let height = row_height(ui);
    let page = rows::Page {
        state,
        data,
        files: &files,
        rows: rows::page_rows(state, data, (ui, rect.width), &file_rows(&files)),
        height,
    };
    let row_text = |index: usize| page.row_text(index);
    let search = ListSearch {
        row_text: &row_text,
        generation: state
            .version
            .wrapping_mul(1_000_003)
            .wrapping_add(u64::try_from(page.rows.len()).unwrap_or(u64::MAX)),
    };
    let reveal = state.reveal.map(|(serial, target)| Reveal {
        serial,
        row: page.row_of(target),
        place: RevealPlace::Top,
    });
    let steering = Steering {
        selection: None,
        reveal,
        left_to_view: &[],
    };
    let list = List {
        name: words::PAGE,
        row_count: page.rows.len(),
        row_height: height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let mut drawn = diff::Drawn::default();
    let mut intents = Vec::new();
    let key = ("commit page rows", state.id);
    let _ = ui.steered_searchable_list(key, (rect, &list), (&search, &steering), |ui, row| {
        intents.extend(page.draw_row(ui, row, &mut drawn));
    });
    intents.extend(diff::sticky_header(ui, &page, (rect, &drawn)));
    intents.extend(diff::back_to_top(ui, &page, (rect, &drawn)));
    intents
}
