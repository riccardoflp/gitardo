//! The rows of the card of the inspector: its head as a block of rows, a row for each file shown, the link that shows
//! them all, and for a stash its untracked files under a heading of their own.

use model::compare::{FileList, ListedFile, NamedCommit};
use model::contract::{FeatureId, Target};
use model::details::CommitDetail;
use model::state::FileChange;
use ui::layout::{Align, Area, place};
use ui::{
    Button, ButtonKind, Label, List, MenuEntry, MenuItem, Rect, Reveal, RevealPlace, SelectionMode, Sense, Steering,
    TextKind, Tone, Ui,
};

use super::files::{CappedList, FileRow, FilesState, directory_row, file_name, file_rows, file_start};
use super::head::{Head, draw_head, head_height};
use super::message::MessageState;
use super::{DetailsRead, DetailsState, FilesOwner, ReadDetails, SelectedDetails, say, show_all_files};
use crate::blocks::{block_top, rows_for};
use crate::feature::FeatureIntent;
use crate::icons;
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

/// The files of the commit of the card, as the inspector lists them: as a tree beside a Diff page
/// (`diff.beside-inspector`).
fn files_of<'a>(state: &DetailsState<'a>, detail: &'a CommitDetail) -> FilesState<'a> {
    FilesState {
        files: &detail.files,
        tree: state.tree,
        layout: if state.shown_file.is_some() {
            super::FileLayout::Tree
        } else {
            state.layout
        },
        folded: state.folded,
        capped: CappedList {
            owner: FilesOwner::Inspector,
            count: detail.files.len(),
            is_all_shown: state.is_all_files_shown,
        },
    }
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
    let files = files_of(state, detail);
    let height = row_height(ui);
    let untracked: &[FileChange] = if is_stash { &detail.untracked_files } else { &[] };
    let is_beside = state.shown_file.is_some();
    let head_rows = if is_beside {
        1
    } else {
        rows_for(
            head_height(ui, &head, rect.width) + ui.theme().sizes.spacing.medium,
            height,
        )
    };
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
    let steering = Steering {
        reveal: reveal_of(state, &rows, (&files, untracked)),
        ..Steering::default()
    };
    let shown = state.shown_file.map(|shown| shown.path);
    let _ = ui.steered_list(key, (rect, &list), &steering, |ui, row| {
        let first = std::mem::replace(&mut is_first, false);
        match rows.get(row.index) {
            Some(CardRow::Head) if is_beside => intents.extend(commit_line(ui, row.rect, read)),
            Some(CardRow::Head) => {
                if let Some(top) = block_top(row, 0, first, height) {
                    intents.extend(draw_head(ui, &head, top));
                }
            }
            Some(CardRow::File(file)) => intents.extend(file_row(ui, row, (&files, file, detail), shown)),
            Some(CardRow::ShowAll) => intents.extend(ui.clip(row.rect, |ui| show_all_files(&files.capped, ui))),
            Some(CardRow::UntrackedHeading) => {
                let said = words::untracked_files(untracked.len());
                let _ = ui.label(row.rect, &Label::new(&said).kind(TextKind::Detail).tone(Tone::Muted));
            }
            Some(CardRow::Untracked(index)) => {
                intents.extend(untracked_row(
                    ui,
                    row,
                    (detail, untracked, *index),
                    (files.layout, shown),
                ));
            }
            None => {}
        }
    });
    intents
}

/// The commit in one line, at the top of the inspector beside a Diff page (`diff.beside-inspector`): its short hash and
/// the title of its message, with who made it and when in its tooltip; a click brings the timeline back, where the
/// inspector is the whole card again (Q507).
fn commit_line(ui: &mut Ui<'_>, rect: Rect, read: &ReadDetails) -> Option<Intent> {
    let theme = *ui.theme();
    let short = read.detail.id.as_ref().map_or("", model::CommitId::short);
    let said = format!("{short}  {}", read.title);
    let who = read.detail.author.as_ref().map_or_else(String::new, |author| {
        format!(
            "{} \u{b7} {}",
            author.name,
            crate::text::dates::full_date(author.date.0)
        )
    });
    let id = ui.id("commit line");
    let response = ui.interact(id, rect, Sense::CLICK);
    if response.hovered {
        ui.fill_rect(rect, theme.colors.list_hover);
    }
    let _ = ui.label(rect, &Label::new(&said).kind(TextKind::Medium));
    ui.describe(id, rect, &ui::Description::new(ui::AccessRole::Button, &said));
    ui.tooltip(&response, &who);
    response.clicked.then_some(Intent::ShowTimeline)
}

/// The row of the file shown beside the inspector, brought into sight once each time another file is shown.
fn reveal_of(
    state: &DetailsState<'_>,
    rows: &[CardRow<'_>],
    (files, untracked): (&FilesState<'_>, &[FileChange]),
) -> Option<Reveal> {
    let shown = state.shown_file?;
    let path_of = |row: &CardRow<'_>| match row {
        CardRow::File(FileRow::File(index, _)) => files.files.get(*index).map(|file| file.path.as_str()),
        CardRow::Untracked(index) => untracked.get(*index).map(|file| file.path.as_str()),
        CardRow::Head | CardRow::File(FileRow::Directory { .. }) | CardRow::ShowAll | CardRow::UntrackedHeading => None,
    };
    let row = rows.iter().position(|row| path_of(row) == Some(shown.path))?;
    Some(Reveal {
        serial: shown.serial,
        row,
        place: RevealPlace::Nearest,
    })
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

/// A row of the untracked files of a stash, marked while the page beside the inspector shows it, with what a click or
/// its buttons ask.
fn untracked_row(
    ui: &mut Ui<'_>,
    row: &ui::Row,
    (detail, untracked, index): (&CommitDetail, &[FileChange], usize),
    (layout, shown): (super::FileLayout, Option<&str>),
) -> Vec<Intent> {
    let Some(file) = untracked.get(index) else {
        return Vec::new();
    };
    let is_marked = shown == Some(file.path.as_str());
    match inspector_file(ui, row, (file, 0, layout, is_marked), ("untracked file", index)) {
        Some(Picked::Diff) => crate::diff::untracked_file(detail, file)
            .map(open_diff)
            .into_iter()
            .collect(),
        Some(Picked::History) => vec![file_history(file)],
        Some(Picked::Compare) => untracked_list(detail)
            .map(|list| compare_file(file, list))
            .into_iter()
            .collect(),
        None => Vec::new(),
    }
}

/// A row of the files: a directory, which folds, or a file, which opens its diff when clicked.
fn file_row(
    ui: &mut Ui<'_>,
    row: &ui::Row,
    (files, file_row, detail): (&FilesState<'_>, &FileRow<'_>, &CommitDetail),
    shown: Option<&str>,
) -> Vec<Intent> {
    match file_row {
        FileRow::Directory { .. } => directory_row(ui, row.rect, (FilesOwner::Inspector, file_row)),
        FileRow::File(index, depth) => {
            let Some(file) = files.files.get(*index) else {
                return Vec::new();
            };
            let is_marked = shown == Some(file.path.as_str());
            match inspector_file(ui, row, (file, *depth, files.layout, is_marked), ("file", *index)) {
                Some(Picked::Diff) => vec![open_diff(crate::diff::commit_file(detail, file))],
                Some(Picked::History) => vec![file_history(file)],
                Some(Picked::Compare) => vec![compare_file(file, commit_list(detail))],
                None => Vec::new(),
            }
        }
    }
}

/// What opens the Diff page of files of the inspector, beside it (`diff.beside-inspector`).
pub(crate) fn open_diff(target: crate::diff::DiffTarget) -> Intent {
    Intent::Diff(crate::diff::DiffIntent::OpenBeside(Box::new(target)))
}

/// What a file of the inspector was picked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picked {
    /// A click on it, which opens its diff.
    Diff,
    /// Its clock, which shows its history (`file-history.file`).
    History,
    /// **Compare with…** in its context menu (`details.compare-file-with`).
    Compare,
}

/// What compares a file of a list with a side asked for (`details.compare-file-with`).
pub(crate) fn compare_file(file: &FileChange, list: FileList) -> Intent {
    let listed = ListedFile {
        file: file.clone(),
        list,
    };
    let run = FeatureIntent::Run(
        FeatureId("details.compare-file-with"),
        Target::ListedFile(Box::new(listed)),
    );
    Intent::Feature(run)
}

/// The list of the files of a commit, named by its short hash; of the uncommitted changes when it has no hash.
pub(crate) fn commit_list(detail: &CommitDetail) -> FileList {
    match &detail.id {
        Some(commit) => FileList::Commit {
            commit: NamedCommit {
                name: commit.short().to_owned(),
                commit: commit.clone(),
            },
            first_parent: detail.first_parent().cloned(),
        },
        None => FileList::Changes,
    }
}

/// The list of the untracked files of a stash, which git keeps in its third parent, named by the stash.
fn untracked_list(detail: &CommitDetail) -> Option<FileList> {
    let untracked = detail.parents.get(2)?.clone();
    let name = detail
        .id
        .as_ref()
        .map_or_else(|| untracked.short().to_owned(), |stash| stash.short().to_owned());
    Some(FileList::Commit {
        commit: NamedCommit {
            name,
            commit: untracked,
        },
        first_parent: None,
    })
}

/// What shows the history of a file of the inspector, by its path in the working tree (`file-history.file`).
fn file_history(file: &FileChange) -> Intent {
    let run = crate::files::FilesPageIntent::Run(model::contract::FeatureId("file-history.file"), file.path.clone());
    Intent::Files(run)
}

/// A file of the inspector: its status, icon and name, lit under the pointer or marked while its diff is shown beside
/// the inspector, with the clock of its history at its end and **Compare with…** in its context menu; what it was
/// picked for.
fn inspector_file(
    ui: &mut Ui<'_>,
    row: &ui::Row,
    (file, depth, layout, is_marked): (&FileChange, usize, super::FileLayout, bool),
    key: (&'static str, usize),
) -> Option<Picked> {
    let theme = *ui.theme();
    let response = ui.interact(ui.id(key), row.rect, Sense::CLICK | Sense::HOVER);
    if is_marked {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_selection);
    } else if response.hovered {
        ui.fill_rounded_rect(row.rect, theme.sizes.radius.medium, theme.colors.list_hover);
    }
    let rest = file_start(ui, row.rect, (file, depth));
    let mut parts = Area::with_gap(rest, theme.sizes.spacing.small);
    let mut picked = response.clicked.then_some(Picked::Diff);
    // The clock stays while the pointer is on it, which takes the hover from the row.
    if response.hovered || ui.pointer().is_some_and(|pointer| row.rect.contains(pointer)) {
        let side = theme.sizes.row_height;
        let history = Button::icon(icons::HISTORY, words::FILE_HISTORY).kind(ButtonKind::Icon);
        let cell = place(parts.right(side), (side, side), (Align::Start, Align::Center));
        if ui.button((key.0, key.1, "history"), cell, &history).clicked {
            picked = Some(Picked::History);
        }
    }
    file_name(ui, parts.rest(), (file, layout));
    let entries = [MenuEntry::Item(MenuItem::new(words::COMPARE_WITH, ()))];
    if ui.context_menu((key.0, key.1, "menu"), &response, &entries).is_some() {
        picked = Some(Picked::Compare);
    }
    picked
}
