//! What the user changes in a Merge page: a side, or both, put in place of a region (`merge-view.take-side`), every
//! change that needs no choice (`merge-view.accept-simple`), what the rules propose (`merge-view.accept-proposals`),
//! each as one edit that `Ctrl+Z` takes back; the base shown or hidden (`merge-view.show-base`); the current region
//! chosen with `F7` and `Shift+F7` (`merge-view.navigate`); and the file marked as resolved
//! (`merge-view.mark-resolved`).

use model::contract::{FeatureId, Target};
use model::merge_view::{Choice, MergeResolution};
use text::conflicts::parse_markers;
use views::feature::FeatureIntent;
use views::merge_view::{MergePageId, MergePageIntent, MergeText};

use super::edit::{Replacement, lines_of, replaced};
use super::{MergePage, Reading, page_mut, rebuild, start_read};
use crate::store::message::Effect;
use crate::store::state::State;

/// Changes a page as the user asked.
pub fn reduce_page(state: &mut State, id: MergePageId, intent: MergePageIntent) -> Vec<Effect> {
    match intent {
        MergePageIntent::Take(..)
        | MergePageIntent::AcceptSimple
        | MergePageIntent::AcceptProposal(_)
        | MergePageIntent::AcceptProposals => choose(state, id, intent),
        MergePageIntent::ShowBase(is_shown) => show_base(state, is_shown),
        MergePageIntent::Next => step(state, id, true),
        MergePageIntent::Previous => step(state, id, false),
        MergePageIntent::Undo => back_or_again(state, id, true),
        MergePageIntent::Redo => back_or_again(state, id, false),
        MergePageIntent::MarkResolved => mark_resolved(state, id),
        MergePageIntent::Cursor(column, line) => {
            if let Some(page) = page_mut(state, id) {
                page.cursor = Some((column, line));
            }
            Vec::new()
        }
        MergePageIntent::OpenAtLine(column, line) => super::lines::open_file_at(state, id, (column, line)),
    }
}

/// A choice: made at once when the regions are those of the result as it is, made once they are otherwise.
fn choose(state: &mut State, id: MergePageId, intent: MergePageIntent) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    if page.text_generation != page.generation {
        page.waiting = Some(intent);
        return Vec::new();
    }
    let Some(text) = page.text.as_deref() else {
        return Vec::new();
    };
    let replacements = replacements(text, intent);
    if replacements.is_empty() {
        return Vec::new();
    }
    let result = replaced(&page.result, replacements);
    edited(state, id, result)
}

/// What a choice replaces in the result.
fn replacements(text: &MergeText, intent: MergePageIntent) -> Vec<Replacement> {
    let lines = text.lines();
    let texts = lines.texts();
    let take = |(region, choice): (usize, Choice)| {
        let region = text.regions.get(region)?;
        region.would_change(&texts, choice).then(|| Replacement {
            lines: region.result.clone(),
            text: region.chosen(&texts, choice).into_iter().map(str::to_owned).collect(),
        })
    };
    let proposal = |place: usize| {
        let proposal = text.proposals.get(place)?;
        let region = text.regions.get(proposal.region)?;
        region.is_unresolved().then(|| Replacement {
            lines: proposal.replaced.clone(),
            text: lines_of(&proposal.text),
        })
    };
    match intent {
        MergePageIntent::Take(region, choice) => take((region, choice)).into_iter().collect(),
        MergePageIntent::AcceptSimple => text.simple.iter().copied().filter_map(take).collect(),
        MergePageIntent::AcceptProposal(place) => proposal(place).into_iter().collect(),
        MergePageIntent::AcceptProposals => (0..text.proposals.len()).filter_map(proposal).collect(),
        _ => Vec::new(),
    }
}

/// Puts a new result in a page, as one edit `Ctrl+Z` takes back, and computes its regions again.
fn edited(state: &mut State, id: MergePageId, result: String) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    if result == page.result {
        return Vec::new();
    }
    page.undone.push(std::mem::replace(&mut page.result, result));
    page.redone.clear();
    page.generation += 1;
    start_read(state, id, Reading::Regions).into_iter().collect()
}

/// `Ctrl+Z` takes the last edit back, `Ctrl+Y` puts it again.
fn back_or_again(state: &mut State, id: MergePageId, is_back: bool) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let (from, to) = if is_back {
        (&mut page.undone, &mut page.redone)
    } else {
        (&mut page.redone, &mut page.undone)
    };
    let Some(result) = from.pop() else {
        return Vec::new();
    };
    to.push(std::mem::replace(&mut page.result, result));
    page.generation += 1;
    page.waiting = None;
    start_read(state, id, Reading::Regions).into_iter().collect()
}

/// Makes the choice that waited for the regions of the result, once they are computed.
pub(super) fn after_regions(state: &mut State, id: MergePageId) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    if page.text_generation != page.generation {
        return Vec::new();
    }
    match page.waiting.take() {
        Some(intent) => choose(state, id, intent),
        None => Vec::new(),
    }
}

/// The clock: the base shown in a fourth column or hidden, in every Merge page, and remembered for the next.
fn show_base(state: &mut State, is_shown: bool) -> Vec<Effect> {
    if state.persisted.merge_shows_base == is_shown {
        return Vec::new();
    }
    state.persisted.merge_shows_base = is_shown;
    for page in &mut state.merge_pages.pages {
        rebuild(page, is_shown);
    }
    vec![Effect::WritePersisted(state.persisted.clone())]
}

/// `F7` or `Shift+F7`: the next or the previous region, centered in every column; from none, the first or the last;
/// past the last, the first, and before the first, the last.
fn step(state: &mut State, id: MergePageId, is_next: bool) -> Vec<Effect> {
    let serial = state.merge_pages.serial + 1;
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let total = page.layout.regions.len();
    if total == 0 {
        return Vec::new();
    }
    let next = match (is_next, page.current) {
        (true, None) => 0,
        (false, None) => total - 1,
        (true, Some(current)) => (current + 1) % total,
        (false, Some(current)) => current.checked_sub(1).unwrap_or(total - 1),
    };
    go_to_region(page, next, serial);
    state.merge_pages.serial = serial;
    Vec::new()
}

/// Makes a region the current one, and brings it to the middle.
pub(super) fn go_to_region(page: &mut MergePage, region: usize, serial: u64) {
    page.current = Some(region);
    if let Some(rows) = page.layout.regions.get(region) {
        page.reveal = Some((serial, rows.start));
    }
}

/// **Mark as resolved**: the result saved when it has changes that are not, then staged.
fn mark_resolved(state: &mut State, id: MergePageId) -> Vec<Effect> {
    let Some(page) = state.merge_pages.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let resolution = MergeResolution {
        path: page.path.clone(),
        unsaved: page.is_unsaved().then(|| page.result.clone().into_bytes()),
        marked: parse_markers(&page.result).map_or(0, |marked| marked.len()),
    };
    let target = Target::MergeResult(Box::new(resolution));
    crate::store::feature::reduce(state, FeatureIntent::Run(FeatureId("merge-view.mark-resolved"), target))
}
