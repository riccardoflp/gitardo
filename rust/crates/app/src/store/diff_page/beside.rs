//! The Diff page beside the inspector (`diff.beside-inspector`): one page for the files of the inspector of a
//! repository, which shows the file clicked in place of the one it showed, and the next or the previous file of the
//! inspector with `Alt+Down` and `Alt+Up`.

use std::collections::BTreeSet;

use model::details::TreeNode;
use model::state::FileChange;
use views::details::{FileLayout, ShownFile};
use views::diff::{DiffPageId, DiffTarget, FileDiff, FileStep};
use views::page::Page;

use super::{DiffPage, lines, page_mut, read_file};
use crate::persisted::same_folder;
use crate::store::inspector::InspectorRead;
use crate::store::message::Effect;
use crate::store::state::State;

/// Opens the diff of a file of the inspector beside it: in the page beside the inspector of the repository shown, in
/// place of what it showed, or in a new page when there is none.
pub fn open(state: &mut State, target: DiffTarget) -> Vec<Effect> {
    let Some(folder) = state.shown_read().map(|read| read.folder.clone()) else {
        return Vec::new();
    };
    state.diff_pages.serial += 1;
    let serial = state.diff_pages.serial;
    let found = state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.beside.is_some() && same_folder(&page.folder, &folder))
        .map(|page| (page.id, page.target == target));
    match found {
        None => super::open_page(state, target, super::Owner::Inspector(serial)),
        Some((id, true)) => {
            show(state, id);
            Vec::new()
        }
        Some((id, false)) => {
            let effects = retarget(state, id, target);
            if let Some(page) = page_mut(state, id) {
                page.beside = Some(serial);
            }
            show(state, id);
            effects
        }
    }
}

/// Shows the page beside the inspector once it is open: a page of one file opens once its file is read.
fn show(state: &mut State, id: DiffPageId) {
    if state.diff_pages.pages.iter().any(|page| page.id == id && page.is_open) {
        super::super::pages::show_page(state, Page::Diff(id));
    }
}

/// Shows another target in a page beside what opened it, which keeps its number, its place among the pages and the
/// width of its sides: the reads of what it showed are cancelled and those of the new files start.
pub fn retarget(state: &mut State, id: DiffPageId, target: DiffTarget) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    let cancelled: Vec<_> = page.jobs.iter().flatten().copied().collect();
    let count = target.files.len();
    page.title = target.page_title();
    page.target = target;
    page.diffs = vec![FileDiff::Loading; count];
    page.jobs = vec![None; count];
    page.unfolded = vec![BTreeSet::new(); count];
    clear_place(page);
    lines::rebuild(page);
    let mut effects: Vec<Effect> = cancelled
        .into_iter()
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .collect();
    effects.extend((0..count).filter_map(|file| read_file(state, id, file, count != 1)));
    effects
}

/// Forgets where a page was in what it showed: the change gone to, the caret, what the bar said and the edits waiting.
fn clear_place(page: &mut DiffPage) {
    page.current = None;
    page.reveal = None;
    page.hint = None;
    page.going = None;
    page.caret = None;
    page.caret_reveal = None;
    page.edited_at = None;
    page.is_stale = false;
}

/// `Alt+Down` or `Alt+Up` in the page beside the inspector: the diff of the next or the previous file of the
/// inspector, in the same page; the first file when the page shows something else, such as all of them.
pub fn step(state: &mut State, step: FileStep) -> Vec<Effect> {
    let Some(Page::Diff(id)) = state.page_shown() else {
        return Vec::new();
    };
    let Some(page) = state
        .diff_pages
        .pages
        .iter()
        .find(|page| page.id == id && page.beside.is_some())
    else {
        return Vec::new();
    };
    let targets = inspector_targets(state);
    let shown = targets.iter().position(|target| *target == page.target);
    let next = match (shown, step) {
        (Some(index), FileStep::Next) => targets.get(index + 1),
        (Some(index), FileStep::Previous) => index.checked_sub(1).and_then(|index| targets.get(index)),
        (None, _) => targets.first(),
    };
    match next.cloned() {
        Some(target) => open(state, target),
        None => Vec::new(),
    }
}

/// The diff of each file of the inspector, in the order it lists them: of the comparison shown in place of the card,
/// or of the row selected, as a list or as a tree, then the untracked files of a stash.
fn inspector_targets(state: &State) -> Vec<DiffTarget> {
    if let Some(in_place) = &state.compares.in_place {
        let files = match &in_place.files {
            Some(Ok(files)) => files.as_slice(),
            _ => &[],
        };
        return (0..files.len())
            .filter_map(|index| files.get(index..=index))
            .map(|file| views::compare::in_place_diff_target(&in_place.sides, file))
            .collect();
    }
    let inspector = &state.inspector;
    let InspectorRead::Read(read) = &inspector.read else {
        return Vec::new();
    };
    let detail = &read.detail;
    let order: Vec<usize> = match state.persisted.file_layouts.inspector {
        FileLayout::List => (0..shown_count(&detail.files, inspector.is_all_files_shown)).collect(),
        FileLayout::Tree => {
            let mut indices = Vec::new();
            tree_files(&inspector.tree, &mut indices);
            indices
        }
    };
    let files = order.iter().filter_map(|index| detail.files.get(*index));
    let untracked = detail
        .untracked_files
        .iter()
        .filter_map(|file| views::diff::untracked_file(detail, file));
    files
        .map(|file| views::diff::commit_file(detail, file))
        .chain(untracked)
        .collect()
}

/// The files of a tree, by their places in the list it was made from, in the order the tree shows them, those of
/// folded directories too.
pub(crate) fn tree_files(nodes: &[TreeNode], indices: &mut Vec<usize>) {
    for node in nodes {
        match node {
            TreeNode::Directory { children, .. } => tree_files(children, indices),
            TreeNode::File { index, .. } => indices.push(*index),
        }
    }
}

/// How many files the inspector lists: the first 300, or all of them once **Show all** was clicked.
fn shown_count(files: &[FileChange], is_all_shown: bool) -> usize {
    if is_all_shown {
        files.len()
    } else {
        files.len().min(views::details::FILE_CAP)
    }
}

/// The file whose diff the page beside the inspector shows, for the inspector to mark it, while that page is shown and
/// shows one file.
#[must_use]
pub fn shown_file(state: &State) -> Option<ShownFile<'_>> {
    let Some(Page::Diff(id)) = state.page_shown() else {
        return None;
    };
    let page = state.diff_pages.pages.iter().find(|page| page.id == id)?;
    let serial = page.beside?;
    match page.target.files.as_slice() {
        [file] => Some(ShownFile {
            path: &file.right,
            serial,
        }),
        _ => None,
    }
}

/// Whether a Diff page is the one beside the inspector.
#[must_use]
pub fn is_beside(state: &State, id: DiffPageId) -> bool {
    state
        .diff_pages
        .pages
        .iter()
        .any(|page| page.id == id && page.beside.is_some())
}
