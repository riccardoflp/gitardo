//! A comparison beside its diffs as the store keeps it (`compare.beside-diff`): one Diff page for the files of a
//! comparison page, shown in that page rather than as a page of its own, which shows the file clicked in place of the
//! one it showed, the next or the previous file with `Alt+Down` and `Alt+Up`, and the same file the other way once
//! the comparison is swapped or read again.

use std::collections::BTreeSet;

use model::details::TreeNode;
use views::compare::{BesideDiffState, ComparePageId, CompareState, diff_target};
use views::diff::{DiffPageId, FileStep};
use views::page::{Page, PageState};

use super::PageRead;
use crate::store::diff_page::{self, Owner};
use crate::store::message::Effect;
use crate::store::state::State;

/// The diff of files of a comparison page, beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Beside {
    /// The Diff page.
    pub diff: DiffPageId,
    /// Whether it is shown, rather than the whole comparison.
    pub is_shown: bool,
    /// The path of the file it shows; none when it shows every file.
    pub shown: Option<String>,
    /// The files of the comparison as a tree.
    pub tree: Vec<TreeNode>,
    /// The folders of the tree closed by the user.
    pub folded: BTreeSet<String>,
    /// Whether **Commits** is unfolded.
    pub are_commits_unfolded: bool,
    /// The number of the last time another file was shown, which brings its row into sight once.
    pub serial: u64,
}

/// A file of a comparison page, or every file it lists, shown in a diff beside it: in the Diff page it has, in place
/// of what that showed, or in a new one.
pub fn open(state: &mut State, id: ComparePageId, index: Option<usize>) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let PageRead::Read(view) = &page.read else {
        return Vec::new();
    };
    let files = match index {
        Some(index) => view.files.get(index..=index).unwrap_or_default(),
        None => &view.files,
    };
    let Some(first) = files.first() else {
        return Vec::new();
    };
    let shown = index.map(|_| first.path.clone());
    let target = diff_target(&page.target, view.merge_base.as_ref(), files);
    let tree = crate::store::inspector::tree_of(&view.files, true);
    state.diff_pages.serial += 1;
    let serial = state.diff_pages.serial;
    let kept = page
        .beside
        .as_ref()
        .filter(|beside| state.diff_pages.pages.iter().any(|diff| diff.id == beside.diff))
        .cloned();
    let Some((diff, effects)) = show_in(state, (id, kept), target, serial) else {
        return Vec::new();
    };
    if let Some(page) = super::page_mut(state, id) {
        page.beside = Some(Beside {
            is_shown: true,
            shown,
            tree,
            serial,
            ..diff
        });
    }
    effects
}

/// Shows a target in the Diff page beside a comparison page, in place of what it showed, or in a new one; gives what
/// the page then keeps of it.
fn show_in(
    state: &mut State,
    (id, kept): (ComparePageId, Option<Beside>),
    target: views::diff::DiffTarget,
    serial: u64,
) -> Option<(Beside, Vec<Effect>)> {
    if let Some(beside) = kept {
        let effects = if diff_of(state, beside.diff).is_some_and(|diff| diff.target == target) {
            Vec::new()
        } else {
            diff_page::beside::retarget(state, beside.diff, target)
        };
        super::super::pages::show_page(state, Page::Compare(id));
        return Some((beside, effects));
    }
    let effects = diff_page::open_page(state, target, Owner::Comparison(id));
    let diff = state
        .diff_pages
        .pages
        .iter()
        .rev()
        .find(|diff| diff.comparison == Some(id))?;
    let beside = Beside {
        diff: diff.id,
        is_shown: true,
        shown: None,
        tree: Vec::new(),
        folded: BTreeSet::new(),
        are_commits_unfolded: false,
        serial,
    };
    Some((beside, effects))
}

/// A Diff page by its number.
fn diff_of(state: &State, id: DiffPageId) -> Option<&diff_page::DiffPage> {
    state.diff_pages.pages.iter().find(|page| page.id == id)
}

/// `Alt+Down` or `Alt+Up` in the diff beside a comparison: the next or the previous file in the order of its tree; the
/// first when it shows every file.
pub fn step(state: &mut State, id: ComparePageId, step: FileStep) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let (Some(beside), PageRead::Read(view)) = (&page.beside, &page.read) else {
        return Vec::new();
    };
    let mut order = Vec::new();
    diff_page::beside::tree_files(&beside.tree, &mut order);
    let shown = beside.shown.as_deref().and_then(|path| {
        order
            .iter()
            .position(|index| view.files.get(*index).is_some_and(|file| file.path == path))
    });
    let next = match (shown, step) {
        (Some(place), FileStep::Next) => order.get(place + 1),
        (Some(place), FileStep::Previous) => place.checked_sub(1).and_then(|place| order.get(place)),
        (None, _) => order.first(),
    };
    match next.copied() {
        Some(index) => open(state, id, Some(index)),
        None => Vec::new(),
    }
}

/// `Escape` in the diff, or the step of the comparison in the trail: the whole comparison page again, as it was
/// scrolled; the Diff page is kept for the next file opened.
pub fn show_whole(state: &mut State, id: ComparePageId) -> Vec<Effect> {
    if let Some(beside) = super::page_mut(state, id).and_then(|page| page.beside.as_mut()) {
        beside.is_shown = false;
    }
    super::super::pages::show_page(state, Page::Compare(id));
    Vec::new()
}

/// **Commits** in the column: the commits folded or unfolded.
pub fn fold_commits(state: &mut State, id: ComparePageId) -> Vec<Effect> {
    if let Some(beside) = super::page_mut(state, id).and_then(|page| page.beside.as_mut()) {
        beside.are_commits_unfolded = !beside.are_commits_unfolded;
    }
    Vec::new()
}

/// A folder of the tree in the column closed, or opened again.
pub fn fold(state: &mut State, id: ComparePageId, path: String) -> Vec<Effect> {
    if let Some(beside) = super::page_mut(state, id).and_then(|page| page.beside.as_mut())
        && !beside.folded.remove(&path)
    {
        let _new = beside.folded.insert(path);
    }
    Vec::new()
}

/// Once a comparison page with a diff beside it is read again, as after its sides were swapped: its tree is made again,
/// and the diff shows the same file, or every file, between the sides as they are now.
pub fn follow_read(state: &mut State, id: ComparePageId) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let (Some(beside), PageRead::Read(view)) = (&page.beside, &page.read) else {
        return Vec::new();
    };
    let tree = crate::store::inspector::tree_of(&view.files, true);
    let files = match &beside.shown {
        Some(path) => view
            .files
            .iter()
            .position(|file| file.path == *path)
            .and_then(|index| view.files.get(index..=index))
            .unwrap_or_default(),
        None => &view.files,
    };
    let target = (!files.is_empty()).then(|| diff_target(&page.target, view.merge_base.as_ref(), files));
    let diff = beside.diff;
    if let Some(beside) = super::page_mut(state, id).and_then(|page| page.beside.as_mut()) {
        beside.tree = tree;
    }
    match target {
        Some(target) if diff_of(state, diff).is_some_and(|page| page.target != target) => {
            diff_page::beside::retarget(state, diff, target)
        }
        _ => Vec::new(),
    }
}

/// A comparison page as it is drawn: with the diff of a file beside it while one is shown, the column as wide as the
/// inspector.
#[must_use]
pub fn page_state(state: &State, id: ComparePageId, column_width: Option<f32>) -> Option<PageState<'_>> {
    let compare = super::page_state(state, id)?;
    Some(match beside_state(state, compare, column_width) {
        Some(beside) => PageState::CompareBeside(Box::new(beside)),
        None => PageState::Compare(compare),
    })
}

/// The comparison page with the diff of its file beside it, while one is shown and its first read came.
#[must_use]
pub fn beside_state<'a>(
    state: &'a State,
    compare: CompareState<'a>,
    column_width: Option<f32>,
) -> Option<BesideDiffState<'a>> {
    let page = state.compares.pages.iter().find(|page| page.id == compare.id)?;
    let beside = page.beside.as_ref().filter(|beside| beside.is_shown)?;
    let diff = diff_of(state, beside.diff).filter(|diff| diff.is_open)?;
    let diff = diff_page::page_state(state, diff.id)?;
    Some(BesideDiffState {
        compare,
        diff,
        tree: &beside.tree,
        folded: &beside.folded,
        marked: beside.shown.as_deref(),
        serial: beside.serial,
        are_commits_unfolded: beside.are_commits_unfolded,
        column_width: column_width.map(crate::store::files_page::whole_pixels),
    })
}
