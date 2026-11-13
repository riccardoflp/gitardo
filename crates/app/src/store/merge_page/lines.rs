//! Where a Merge page is in its file: opened at a line of the file on disk, as **Resolve by hand** on a card of the
//! review and the line number of a card ask (`conflicts.open-file`), the region of the result that line is in brought
//! to the middle.

use model::merge_view::Region;
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

/// Brings the row of a line of the result to the middle.
fn reveal_line(page: &mut MergePage, line: usize, serial: u64) {
    if let Some(row) = page.layout.rows.iter().position(|row| row.result == Some(line)) {
        page.reveal = Some((serial, row));
    }
}
