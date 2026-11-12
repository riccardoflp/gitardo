//! The rows of the card of the inspector: its head as a block of rows, a row for each file shown, the link that shows
//! them all, and for a stash its untracked files under a heading of their own.

use model::details::CommitDetail;
use model::state::FileChange;
use ui::layout::{Align, Area, place};
use ui::{Button, ButtonKind, Icon, Label, List, MenuEntry, MenuItem, Rect, SelectionMode, Sense, TextKind, Tone, Ui};

use super::files::{CappedList, FileRow, FilesState, directory_row, file_name, file_rows, file_start};
use super::head::{Head, draw_head, head_height};
use super::message::MessageState;
use super::{DetailsRead, DetailsState, FilesOwner, ReadDetails, SelectedDetails, say, show_all_files};
use crate::blocks::{block_top, rows_for};
use crate::intent::Intent;
use crate::text::details as words;

/// A row of the card.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CardRow<'a> {
    /// A row of the head, which is drawn once from the first of them in sight.
    Head,
    /// A directory or a file.
    File(FileRow<'a>),
    /// **Show all N files…**.
    ShowAll,
    /// **N untracked files**, over the untracked files of a stash.
    UntrackedHeading,
    /// An untracked file of a stash, by its place among them.
    Untracked(usize),
}

/// The card of the row selected, over the rectangle its caller clipped it to: what it says while the details are read
/// or could not be, or its rows.
pub(super) fn card(state: &DetailsState<'_>, ui: &mut Ui<'_>, is_stash: bool) -> Vec<Intent> {
    let rect = ui.current_clip();
    match state.read {
        DetailsRead::Loading => say(ui, rect, words::LOADING),
        DetailsRead::Missing => say(ui, rect, words::NOT_IN_REPOSITORY),
        DetailsRead::Failed => say(ui, rect, words::COULD_NOT_READ),
        DetailsRead::Read(read) => return read_card(state, (read, is_stash), ui, rect),
    }
    Vec::new()
}

/// The height of a row of the card.
fn row_height(ui: &Ui<'_>) -> f32 {
    ui.theme().sizes.row_height + ui.theme().sizes.spacing.small
}

/// The rows of the card of details read.
fn read_card(
    state: &DetailsState<'_>,
    (read, is_stash): (&ReadDetails, bool),
    ui: &mut Ui<'_>,
    rect: Rect,
) -> Vec<Intent> {
    let detail = &read.detail;
    let head = head_of(state, (read, is_stash));
    let files = FilesState {
        files: &detail.files,
        tree: state.tree,
        layout: state.layout,
        folded: state.folded,
        capped: CappedList {
            owner: FilesOwner::Inspector,
            count: detail.files.len(),
            is_all_shown: state.is_all_files_shown,
        },
    };
    let height = row_height(ui);
    let untracked: &[FileChange] = if is_stash { &detail.untracked_files } else { &[] };
    let head_rows = rows_for(
        head_height(ui, &head, rect.width) + ui.theme().sizes.spacing.medium,
        height,
    );
    let rows = card_rows(head_rows, &files, untracked);
    let list = List {
        name: words::FILES,
        row_count: rows.len(),
        row_height: height,
        selection_mode: SelectionMode::Read,
        lead: 0.0,
    };
    let mut intents = Vec::new();
    let mut is_first = true;
    let key = ("inspector rows", detail.id.as_ref().map(model::CommitId::as_str));
    let _ = ui.list(key, rect, &list, |ui, row| {
        let first = std::mem::replace(&mut is_first, false);
        match rows.get(row.index) {
            Some(CardRow::Head) => {
                if let Some(top) = block_top(row, 0, first, height) {
                    intents.extend(draw_head(ui, &head, top));
                }
            }
            Some(CardRow::File(file)) => intents.extend(file_row(ui, row, (&files, file, detail))),
            Some(CardRow::ShowAll) => intents.extend(ui.clip(row.rect, |ui| show_all_files(&files.capped, ui))),
            Some(CardRow::UntrackedHeading) => {
                let said = words::untracked_files(untracked.len());
                let _ = ui.label(row.rect, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
            }
            Some(CardRow::Untracked(index)) => {
                if let Some(file) = untracked.get(*index)
                    && inspector_file(ui, row, (file, 0, files.layout), ("untracked file", *index))
                    && let Some(target) = crate::diff::untracked_file(detail, file)
                {
                    intents.push(open_diff(target));
                }
            }
            None => {}
        }
    });
    intents
}

/// What the head of the card shows.
fn head_of<'a>(state: &DetailsState<'_>, (read, is_stash): (&'a ReadDetails, bool)) -> Head<'a> {
    let detail = &read.detail;
    Head {
        title: &read.title,
        message: MessageState {
            text: &read.text,
            is_expanded: state.is_message_expanded,
        },
        detail: (!matches!(state.selected, Some(SelectedDetails::Draft(_)))).then_some(detail),
        read: detail,
        files: (detail.files.len(), state.layout),
        is_stash,
    }
}

/// The rows of a card: its head, its files, the link that shows them all, and the untracked files of a stash.
fn card_rows<'a>(head_rows: usize, files: &FilesState<'a>, untracked: &[FileChange]) -> Vec<CardRow<'a>> {
    let mut rows = vec![CardRow::Head; head_rows];
    rows.extend(file_rows(files).into_iter().map(CardRow::File));
    if files.capped.is_cut() {
        rows.push(CardRow::ShowAll);
    }
    if !untracked.is_empty() {
        rows.push(CardRow::UntrackedHeading);
        rows.extend((0..untracked.len()).map(CardRow::Untracked));
    }
    rows
}

/// A row of the files: a directory, which folds, or a file, which opens its diff when clicked.
fn file_row(
    ui: &mut Ui<'_>,
    row: &ui::Row,
    (files, file_row, detail): (&FilesState<'_>, &FileRow<'_>, &CommitDetail),
) -> Vec<Intent> {
    match file_row {
        FileRow::Directory { .. } => directory_row(ui, row.rect, (FilesOwner::Inspector, file_row)),
        FileRow::File(index, depth) => match files.files.get(*index) {
            Some(file) if inspector_file(ui, row, (file, *depth, files.layout), ("file", *index)) => {
                vec![open_diff(crate::diff::commit_file(detail, file))]
            }
            _ => Vec::new(),
        },
    }
}

/// What opens the Diff page of a target.
pub(crate) fn open_diff(target: crate::diff::DiffTarget) -> Intent {
    Intent::Diff(crate::diff::DiffIntent::Open(Box::new(target)))
}

/// A file of the inspector: its status, icon and name, lit under the pointer, with the button of its history at its
/// end and **Compare with…** in its context menu; whether it was clicked, which opens its diff. The history of a file
/// and the comparison come in later milestones: their entries are shown and cannot be used yet (Q89).
fn inspector_file(
    ui: &mut Ui<'_>,
    row: &ui::Row,
    (file, depth, layout): (&FileChange, usize, super::FileLayout),
    key: (&'static str, usize),
) -> bool {
    let theme = *ui.theme();
    let response = ui.interact(ui.id(key), row.rect, Sense::CLICK | Sense::HOVER);
    if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let rest = file_start(ui, row.rect, (file, depth));
    let mut parts = Area::with_gap(rest, theme.sizes.spacing.small);
    if response.hovered {
        let side = theme.sizes.row_height;
        let history = Button::icon(Icon::Refresh, words::FILE_HISTORY)
            .kind(ButtonKind::Icon)
            .disabled();
        let cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        let _ = ui.button((key.0, key.1, "history"), cell, &history);
    }
    file_name(ui, parts.rest(), (file, layout));
    let entries = [MenuEntry::Item(MenuItem::new(words::COMPARE_WITH, ()).disabled())];
    let _ = ui.context_menu((key.0, key.1, "menu"), &response, &entries);
    response.clicked
}
