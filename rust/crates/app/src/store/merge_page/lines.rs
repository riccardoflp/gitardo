//! Where a Merge page is in its file: opened at a line of the file on disk, as **Resolve by hand** on a card of the
//! review and the line number of a card ask (`conflicts.open-file`), the region of the result that line is in brought
//! to the middle; the File page opened at the line of the result a line of a column stands for
//! (`merge-view.open-at-line`); and the File page shown in place of a page closed with choices not saved
//! (`merge-view.close`).

use model::merge_view::{Column, Region, result_line};
use views::file_view::FileTarget;
use views::merge_view::MergePageId;

use super::{MergePage, page_mut};
use crate::persisted::same_folder;
use crate::store::message::Effect;
use crate::store::state::State;

/// Opens the Merge page of a file of the repository shown, or shows the one it has, at a line of the file on disk, from
/// 1: the region of the result it is in is made the current one and brought to the middle, once the file is read.
pub fn open_at(state: &mut State, path: &str, line: Option<usize>) -> Vec<Effect> {
    let effects = super::open(state, path.to_owned());
    let (Some(line), Some(folder)) = (line, state.shown_folder().cloned()) else {
        return effects;
    };
    let found = state
        .merge_pages
        .pages
        .iter()
        .find(|page| page.path == path && same_folder(&page.folder, &folder))
        .map(|page| page.id);
    if let Some(id) = found {
        if let Some(page) = page_mut(state, id) {
            page.pending_line = Some(line.saturating_sub(1));
        }
        go_to_pending(state, id);
    }
    effects
}

/// Goes to the line a page was asked to open at, once its regions are known: the region of the result it is in, or the
/// row of that line between regions.
pub(super) fn go_to_pending(state: &mut State, id: MergePageId) {
    let serial = state.merge_pages.serial + 1;
    let Some(page) = page_mut(state, id) else {
        return;
    };
    if page.text.is_none() {
        return;
    }
    let Some(line) = page.pending_line.take() else {
        return;
    };
    match page.text.as_ref().and_then(|text| region_at(&text.regions, line)) {
        Some(region) => super::choose::go_to_region(page, region, serial),
        None => reveal_line(page, line, serial),
    }
    state.merge_pages.serial = serial;
}

/// The region of the result a line of the result is in; a region that took every line out is at the line after it.
fn region_at(regions: &[Region], line: usize) -> Option<usize> {
    regions
        .iter()
        .position(|region| region.result.contains(&line) || (region.result.is_empty() && region.result.start == line))
}

/// `F4` or **Jump to source** on a line of a column: the File page of the file at the line of the result it stands for,
/// holding the result as the page has it when it has choices not saved.
pub(super) fn open_file_at(state: &mut State, id: MergePageId, at: (Column, usize)) -> Vec<Effect> {
    let Some(page) = state.merge_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let Some(text) = page.text.as_deref() else {
        return Vec::new();
    };
    let line = result_line(&text.regions, at);
    let (path, unsaved) = (page.path.clone(), page.is_unsaved().then(|| page.result.clone()));
    in_file_page(state, &path, (unsaved, line))
}

/// A Merge page closed with choices not saved: the File page of its file shows them, not saved, at the line of the
/// result its cursor or its region chosen stands for, so that they are not lost. A page with nothing unsaved opens
/// nothing; a page closed by **Mark as resolved**, or because its file is no longer in conflict, does not come here.
///
/// Feature: merge-view.close
pub(super) fn closed(state: &mut State, page: &MergePage) -> Vec<Effect> {
    let is_shown = state
        .shown_folder()
        .is_some_and(|folder| same_folder(folder, &page.folder));
    if !page.is_unsaved() || !is_shown {
        return Vec::new();
    }
    let line = page.text.as_deref().map_or(0, |text| {
        let current = page.current.and_then(|current| text.regions.get(current));
        page.cursor
            .or_else(|| current.map(|region| (Column::Result, region.result.start)))
            .map_or(0, |at| result_line(&text.regions, at))
    });
    in_file_page(state, &page.path, (Some(page.result.clone()), line))
}

/// The File page of a file of the working tree at a line of the result, from 0, given the result when it is not saved.
fn in_file_page(state: &mut State, path: &str, (unsaved, line): (Option<String>, usize)) -> Vec<Effect> {
    match unsaved {
        Some(text) => crate::store::file_page::open_with_text(state, path, (text, line + 1)),
        None => crate::store::file_page::open(
            state,
            FileTarget {
                path: path.to_owned(),
                revision: None,
                line: Some(line + 1),
            },
        ),
    }
}

/// Brings the row of a line of the result to the middle.
fn reveal_line(page: &mut MergePage, line: usize, serial: u64) {
    if let Some(row) = page.layout.rows.iter().position(|row| row.result == Some(line)) {
        page.reveal = Some((serial, row));
    }
}
