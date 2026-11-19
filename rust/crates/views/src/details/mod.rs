//! The inspector, beside the timeline or over it: the card of the selected commit (`details.show`), its message cut
//! by **Show more** (`details.show-more`), its files as a list or a tree (`details.file-tree`), the first 300 of them
//! (`details.show-all-files`), for a stash the untracked files it took (`details.stash`), and the note of the commit
//! under its message (`note.show`).
//!
//! The card is a list of rows of one height, so that only the rows in sight are drawn: its head (the message, who made
//! the commit, its hash and parents, and the count of its files) is a block of several rows, then a row for each file.

mod files;
mod head;
mod message;
mod note;
mod rows;
mod signature;

use std::collections::BTreeSet;

use model::CommitId;
use model::commit::Commit;
use model::details::{CommitDetail, TreeNode};
use ui::layout::{Insets, inset};
use ui::{AccessRole, Rect, TextKind, Tone, Ui};

use crate::graph::DraftState;
use crate::intent::Intent;
use crate::text::details as words;
use crate::text::timeline as timeline_words;

pub use files::{CappedList, FILE_CAP, FileLayout, FilesIntent, FilesOwner, FilesState, show_all_files};
pub(crate) use files::{
    FileRow, directory_row, file_name, file_rows, file_start, indent_of, layout_buttons, status, tree_file_rows,
};
pub(crate) use head::{outdent, shown_parents};
pub use message::{MessageState, SHOWN_LINES, show_more};
pub use note::{NOTE_SHOWN_LINES, NoteLength, NoteState, note_height, note_view};
pub(crate) use rows::{commit_list, compare_file};
pub use signature::{SignatureRead, SignatureState, signature_view};

/// What the inspector shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DetailsState<'a> {
    /// The row selected in the timeline; none while nothing is.
    pub selected: Option<SelectedDetails<'a>>,
    /// The details read of the row selected.
    pub read: DetailsRead<'a>,
    /// How the files are laid out, as last chosen.
    pub layout: FileLayout,
    /// Whether **Show more** was clicked.
    pub is_message_expanded: bool,
    /// Whether **Show more** of the note was clicked (`note.show`).
    pub is_note_expanded: bool,
    /// Whether **Show all** was clicked.
    pub is_all_files_shown: bool,
    /// The tree of the files shown.
    pub tree: &'a [TreeNode],
    /// The directories of the tree folded.
    pub folded: &'a BTreeSet<String>,
    /// The comparison of two commits shown in place of the card (`compare.in-place`), while it is.
    pub in_place: Option<crate::compare::InPlaceState<'a>>,
    /// The file whose diff is shown beside the inspector, which it marks (`diff.beside-inspector`).
    pub shown_file: Option<ShownFile<'a>>,
    /// The signature of the commit shown (`details.signature`); none for the draft, a stash, or before it is asked.
    pub signature: Option<SignatureRead<'a>>,
    /// The commits whose parents were not fetched, where the history of a shallow clone ends (`graph.shallow-end`).
    pub shallow_ends: &'a [CommitId],
}

/// The file whose diff the Diff page beside the inspector shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShownFile<'a> {
    /// Its path, as the inspector lists it.
    pub path: &'a str,
    /// A number that changes each time another file is shown, for the inspector to bring it into sight once.
    pub serial: u64,
}

/// The row selected, as the timeline read it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SelectedDetails<'a> {
    /// The draft of the uncommitted changes.
    Draft(DraftState),
    /// A commit.
    Commit(&'a Commit),
    /// A stash.
    Stash {
        /// The commit git keeps it as.
        commit: &'a Commit,
        /// Its place among the stashes.
        index: usize,
    },
}

/// How far the details of the row selected are read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailsRead<'a> {
    /// They are read.
    Loading,
    /// They are read.
    Read(&'a ReadDetails),
    /// The commit is not in the repository any more.
    Missing,
    /// Git could not read them.
    Failed,
}

/// The details of a commit as read, with its message shaped for the inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadDetails {
    /// What git said of the commit.
    pub detail: CommitDetail,
    /// The title of its message (`details.message-title`).
    pub title: String,
    /// The text under the title, unwrapped (`details.message-unwrap`).
    pub text: String,
}

/// Something asked of the inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DetailsIntent {
    /// Close the inspector: nothing is selected any more.
    Close,
    /// Open the commit shown as a page (`commit-page.open`).
    OpenPage(CommitId),
    /// **Show more** or **Show less**.
    ShowMore(bool),
    /// **Show more** or **Show less** under the note (`note.show`).
    ShowMoreNote(bool),
    /// Something asked of its files.
    Files(FilesIntent),
}

/// The inspector, along the rectangle its caller clipped it to: a card with the details of the row selected, or what
/// it says while nothing is.
///
/// Feature: details.show
pub fn show(state: &DetailsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    // The column is the island of the inspector (`graph.islands`): the card is the island itself.
    let card = ui.current_clip();
    let theme = *ui.theme();
    let padding = theme.sizes.spacing.large + theme.sizes.spacing.small;
    let inner = inset(card, Insets::symmetric(padding, padding));
    let mut intents = Vec::new();
    ui.region(
        "inspector region",
        (card, AccessRole::Complementary, timeline_words::INSPECTOR),
        |ui| {
            intents = ui.clip(inner, |ui| match (&state.in_place, state.selected) {
                (Some(in_place), _) => crate::compare::in_place(in_place, ui),
                (None, Some(SelectedDetails::Stash { .. })) => stash(state, ui),
                (None, Some(_)) => rows::card(state, ui, false),
                (None, None) => {
                    say(ui, inner, words::NOTHING_SELECTED);
                    Vec::new()
                }
            });
        },
    );
    intents
}

/// The inspector of a stash, over the rectangle its caller clipped it to: its message, author, date and hash as a
/// commit has them, the files it changed against the commit it was made on, and the untracked files it took under
/// **N untracked files**, apart from the others.
///
/// Feature: details.stash
pub fn stash(state: &DetailsState<'_>, ui: &mut Ui<'_>) -> Vec<Intent> {
    rows::card(state, ui, true)
}

/// A sentence alone at the top of the card, in the muted tone.
pub(crate) fn say(ui: &mut Ui<'_>, rect: Rect, said: &str) {
    let _ = ui.paragraph(rect, said, (TextKind::Body, Tone::Muted));
}
