//! What the user asks of a comparison page, and of the comparison in the inspector.

use std::sync::Arc;

use model::compare::{CompareMode, ComparisonPart};
use model::contract::{Answers, FeatureId, Target};
use views::compare::{CompareIntent, ComparePageId, diff_target, narrowed_path};

use super::{PageRead, read_again, remember, title_of};
use crate::store::message::Effect;
use crate::store::state::State;

/// Something asked of a comparison page or of the comparison in the inspector.
pub fn reduce(state: &mut State, intent: CompareIntent) -> Vec<Effect> {
    match intent {
        CompareIntent::ChangeSide(id, is_first) => {
            let part = if is_first {
                ComparisonPart::FirstSide
            } else {
                ComparisonPart::SecondSide
            };
            let effects = run(state, id, "compare.change-side", part);
            if !effects.is_empty() {
                state.compares.changing = Some(id);
            }
            effects
        }
        CompareIntent::Swap(id) => swap(state, id),
        CompareIntent::SetMode(id, mode) => set_mode(state, id, mode),
        CompareIntent::TypePath(id, text) => {
            if let Some(page) = page_mut(state, id) {
                page.typed = text;
            }
            Vec::new()
        }
        CompareIntent::Narrow(id) => {
            let typed = page_mut(state, id).map(|page| page.typed.clone()).unwrap_or_default();
            narrow(state, id, narrowed_path(&typed))
        }
        CompareIntent::ResetPath(id) => {
            if let Some(page) = page_mut(state, id) {
                page.typed = page.target.path.clone().unwrap_or_default();
            }
            Vec::new()
        }
        CompareIntent::ClearPath(id) => narrow(state, id, None),
        CompareIntent::OpenCommit(id, commit) => match page_mut(state, id) {
            Some(page) => {
                let folder = page.folder.clone();
                crate::store::commit_page::open(state, folder, commit)
            }
            None => Vec::new(),
        },
        CompareIntent::ShowAllCommits(id) => {
            if let Some(page) = page_mut(state, id) {
                page.is_all_commits_shown = true;
            }
            Vec::new()
        }
        CompareIntent::OpenFile(id, index) => open_files(state, id, Some(index)),
        CompareIntent::OpenAll(id) => open_files(state, id, None),
        CompareIntent::TakeFile(id, index) => take_file(state, id, index),
        CompareIntent::CopyPatch(id) => run(state, id, "compare.copy-patch", ComparisonPart::Whole),
        CompareIntent::OpenOnGitHub(id) => run(state, id, "compare.open-on-github", ComparisonPart::Whole),
        CompareIntent::CloseInPlace
        | CompareIntent::SwapInPlace
        | CompareIntent::InPlaceAsPage
        | CompareIntent::InPlaceFile(_)
        | CompareIntent::InPlaceAll => super::in_place::reduce(state, &intent),
    }
}

/// The arrow of a file: `compare.take-file` on it.
fn take_file(state: &mut State, id: ComparePageId, index: usize) -> Vec<Effect> {
    let file = match state
        .compares
        .pages
        .iter()
        .find(|page| page.id == id)
        .map(|page| &page.read)
    {
        Some(PageRead::Read(view)) => view.files.get(index).cloned(),
        _ => None,
    };
    match file {
        Some(file) => run(state, id, "compare.take-file", ComparisonPart::File(file)),
        None => Vec::new(),
    }
}

/// A page, to change.
fn page_mut(state: &mut State, id: ComparePageId) -> Option<&mut super::ComparePage> {
    state.compares.pages.iter_mut().find(|page| page.id == id)
}

/// Runs an operation of the catalog on a part of a page, in the repository of the page as last read.
fn run(state: &mut State, id: ComparePageId, feature: &'static str, part: ComparisonPart) -> Vec<Effect> {
    let Some(page) = state.compares.pages.iter().find(|page| page.id == id) else {
        return Vec::new();
    };
    let repository = (page.folder.clone(), Arc::clone(&page.repository));
    let target = Target::Comparison(Box::new(page.target_of(part)));
    crate::store::feature::run_in(state, repository, (FeatureId(feature), target), Answers::new())
}

/// The arrows between the sides: the other direction, the mode, the path and the counts kept, remembered as the latest.
fn swap(state: &mut State, id: ComparePageId) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    std::mem::swap(&mut page.target.first, &mut page.target.second);
    page.whole = page.whole.map(|(first, second)| (second, first));
    page.title = title_of(&page.target);
    let (folder, target) = (page.folder.clone(), page.target.clone());
    remember(state, &folder, &target);
    read_again(state, id)
}

/// A mode button: the page compared again in that mode.
fn set_mode(state: &mut State, id: ComparePageId, mode: CompareMode) -> Vec<Effect> {
    match page_mut(state, id) {
        Some(page) if page.target.mode != mode => {
            page.target.mode = mode;
            read_again(state, id)
        }
        _ => Vec::new(),
    }
}

/// The page narrowed to a path, or to none: retitled, remembered and compared again.
fn narrow(state: &mut State, id: ComparePageId, path: Option<String>) -> Vec<Effect> {
    let Some(page) = page_mut(state, id) else {
        return Vec::new();
    };
    page.typed = path.clone().unwrap_or_default();
    if page.target.path == path {
        return Vec::new();
    }
    page.target.path = path;
    page.title = title_of(&page.target);
    let (folder, target) = (page.folder.clone(), page.target.clone());
    remember(state, &folder, &target);
    read_again(state, id)
}

/// A file of a page, or every file listed, in one Diff page between the two sides as the mode says.
fn open_files(state: &mut State, id: ComparePageId, index: Option<usize>) -> Vec<Effect> {
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
    if files.is_empty() {
        return Vec::new();
    }
    let target = diff_target(&page.target, view.merge_base.as_ref(), files);
    crate::store::diff_page::open(state, target)
}
