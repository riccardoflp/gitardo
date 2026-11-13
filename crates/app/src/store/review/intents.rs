//! What the review page asks of the store: accepting and taking back (`review.accept`, `review.accept-file`,
//! `review.accept-all`, `review.clear`), the choices (`review.choose`), what is shown, the result
//! (`review.show-result`), the write (`review.write`) and what follows it (`review.report`).

use std::sync::Arc;

use model::contract::{Answers, FeatureId, Target};
use model::review::{FileResolutions, Resolutions};
use views::diff::{DiffFile, DiffTarget, DiffVersion};
use views::review::{ConflictKey, Filter, Report, ReviewIntent};

use super::cards::file_proposals;
use super::{Fingerprinted, ReviewPage, derive, read_again};
use crate::store::message::Effect;
use crate::store::state::State;

/// The operation that writes the accepted resolutions.
pub const WRITE: &str = "review.write";
/// The operation that continues the operation that stopped, once it is built.
pub const CONTINUE: &str = "conflicts.continue-operation";
/// The operation that puts back the conflicts of the files written, once it is built.
pub const RESTORE: &str = "conflicts.restore";

/// A conflict by its file and fingerprint.
fn fingerprinted(key: &ConflictKey) -> Fingerprinted {
    (key.path.clone(), key.fingerprint.clone())
}

/// What the review page asks.
pub fn reduce(state: &mut State, intent: ReviewIntent) -> Vec<Effect> {
    match intent {
        ReviewIntent::ShowResult => return show_result(state),
        ReviewIntent::Write => return write(state),
        ReviewIntent::Undo => return undo(state),
        ReviewIntent::Continue => return run(state, CONTINUE, Target::Head),
        ReviewIntent::Layout(layout) => state.reviews.layout = layout,
        ReviewIntent::Widths(widths) => state.reviews.widths = widths,
        ReviewIntent::Choose(key, kind) => {
            let Some(folder) = state.reviews.page.as_ref().map(|page| page.folder.clone()) else {
                return Vec::new();
            };
            let known = fingerprinted(&key);
            state.reviews.choices.retain(|(_, choice, _)| *choice != known);
            state.reviews.choices.push((folder, known, kind));
            if let Some(page) = state.reviews.page.as_mut() {
                accept(page, &key);
                let _ = page.folds.insert(fingerprinted(&key), false);
            }
        }
        ReviewIntent::DropChoice(key) => {
            let known = fingerprinted(&key);
            state.reviews.choices.retain(|(_, choice, _)| *choice != known);
            if let Some(page) = state.reviews.page.as_mut() {
                let _ = page.accepted.remove(&known);
            }
        }
        intent => {
            if let Some(page) = state.reviews.page.as_mut() {
                change_page(page, intent);
            }
        }
    }
    derive(state);
    Vec::new()
}

/// Accepts a conflict: folds it, and keeps it in view while **Not accepted** is on.
fn accept(page: &mut ReviewPage, key: &ConflictKey) {
    let known = fingerprinted(key);
    let _ = page.accepted.insert(known.clone());
    let _ = page.folds.remove(&known);
    let _ = page.file_folds.remove(&key.path);
    page.report = None;
    if page.filter == Filter::NotAccepted && !page.kept_in_view.contains(key) {
        page.kept_in_view.push(key.clone());
    }
}

/// A change of what the page has accepted, or shows.
fn change_page(page: &mut ReviewPage, intent: ReviewIntent) {
    match intent {
        ReviewIntent::Accept(key) => accept(page, &key),
        ReviewIntent::TakeBack(key) => {
            let known = fingerprinted(&key);
            let _ = page.accepted.remove(&known);
            let _ = page.folds.remove(&known);
            let _ = page.file_folds.remove(&key.path);
        }
        ReviewIntent::AcceptFile(path) => {
            for key in acceptable(page, Some(&path)) {
                accept(page, &key);
            }
        }
        ReviewIntent::AcceptAll => {
            for key in acceptable(page, None) {
                accept(page, &key);
            }
        }
        ReviewIntent::Clear => {
            page.accepted.clear();
            page.folds.clear();
            page.file_folds.clear();
        }
        ReviewIntent::ToggleSides(key) => toggle(&mut page.sides_shown, fingerprinted(&key)),
        ReviewIntent::ToggleBase(key) => toggle(&mut page.base_shown, fingerprinted(&key)),
        ReviewIntent::Fold(key, is_folded) => {
            let _ = page.folds.insert(fingerprinted(&key), is_folded);
        }
        ReviewIntent::FoldFile(path, is_folded) => {
            let _ = page.file_folds.insert(path, is_folded);
        }
        ReviewIntent::Filter(filter) => {
            page.filter = filter;
            page.kept_in_view.clear();
        }
        ReviewIntent::ShowFile(path) => page.file_shown = path,
        _ => {}
    }
}

/// Adds a conflict to a set, or takes it out.
fn toggle(set: &mut std::collections::BTreeSet<Fingerprinted>, known: Fingerprinted) {
    if !set.remove(&known) {
        let _ = set.insert(known);
    }
}

/// The conflicts of a file, or of the page, whose proposal could be accepted with the others.
fn acceptable(page: &ReviewPage, path: Option<&str>) -> Vec<ConflictKey> {
    page.cards
        .iter()
        .filter(|file| path.is_none_or(|path| path == file.path))
        .flat_map(|file| &file.conflicts)
        .filter(|card| card.is_acceptable_with_others())
        .map(|card| card.key.clone())
        .collect()
}

/// The accepted resolutions of every file of the page.
fn resolutions(state: &State) -> Option<Resolutions> {
    let page = state.reviews.page.as_ref()?;
    let readout = page.readout.as_deref()?;
    let choices: Vec<(Fingerprinted, model::review::ChoiceKind)> = state
        .reviews
        .choices
        .iter()
        .map(|(_, known, kind)| (known.clone(), *kind))
        .collect();
    let settings = (
        state.settings.features.conflicts_auto_resolve,
        state.settings.features.conflicts_ai,
    );
    let files = readout
        .read
        .files
        .iter()
        .map(|file| {
            let accepted = file
                .conflicts
                .iter()
                .zip(file_proposals(file, &choices, settings))
                .filter(|(conflict, _)| {
                    page.accepted
                        .contains(&(file.path.clone(), conflict.fingerprint.clone()))
                })
                .filter_map(|(_, proposal)| proposal)
                .collect();
            FileResolutions {
                path: file.path.clone(),
                text: file.text.clone(),
                conflicts: file.conflicts.len(),
                accepted,
            }
        })
        .collect();
    Some(Resolutions { files })
}

/// What the report will say once the write ran.
fn report_of(resolutions: &Resolutions, page: &ReviewPage) -> Report {
    let written: Vec<&FileResolutions> = resolutions
        .files
        .iter()
        .filter(|file| !file.accepted.is_empty())
        .collect();
    let others = page
        .readout
        .as_deref()
        .map_or(0, |readout| readout.read.deleted.len() + readout.read.others.len());
    Report {
        conflicts: resolutions.accepted(),
        files: written.len(),
        from_ai: resolutions.from_ai(),
        staged: written.iter().filter(|file| file.is_whole()).count(),
        conflicts_left: written.iter().map(|file| file.conflicts - file.accepted.len()).sum(),
        files_left: resolutions.files.iter().filter(|file| !file.is_whole()).count() + others,
    }
}

/// Runs `review.write` on the accepted resolutions.
fn write(state: &mut State) -> Vec<Effect> {
    let Some(resolutions) = resolutions(state) else {
        return Vec::new();
    };
    let Some(page) = state.reviews.page.as_mut() else {
        return Vec::new();
    };
    page.writing = Some(report_of(&resolutions, page));
    run(state, WRITE, Target::Resolutions(Box::new(resolutions)))
}

/// Runs an operation in the repository of the page.
fn run(state: &mut State, feature: &'static str, target: Target) -> Vec<Effect> {
    let Some(page) = state.reviews.page.as_ref() else {
        return Vec::new();
    };
    if features::operation(feature).is_none() {
        return Vec::new();
    }
    let repository = (page.folder.clone(), Arc::clone(&page.repository));
    crate::store::feature::run_in(state, repository, (FeatureId(feature), target), Answers::new())
}

/// **Undo** in the report: `conflicts.restore` once it is built, else `undo.last`, which puts back the conflicts of the
/// files written from the journal.
fn undo(state: &mut State) -> Vec<Effect> {
    if let Some(page) = state.reviews.page.as_mut() {
        page.report = None;
    }
    if features::operation(RESTORE).is_some() {
        run(state, RESTORE, Target::Head)
    } else {
        run(state, "undo.last", Target::Head)
    }
}

/// Opens one Diff page with every file that has an accepted resolution: the file on disk beside the file with them in
/// place.
fn show_result(state: &mut State) -> Vec<Effect> {
    let Some(resolutions) = resolutions(state) else {
        return Vec::new();
    };
    let resolved: Vec<(String, String)> = resolutions
        .files
        .iter()
        .filter(|file| !file.accepted.is_empty())
        .filter_map(|file| file.resolved().ok().map(|text| (file.path.clone(), text)))
        .collect();
    if resolved.is_empty() {
        return Vec::new();
    }
    let target = DiffTarget {
        left: DiffVersion::WorkingTree,
        files: resolved.iter().map(|(path, _)| DiffFile::same(path)).collect(),
        right: DiffVersion::Resolved(resolved),
        title: Some(String::from(views::text::review::RESULT_PAGE)),
    };
    crate::store::diff_page::open(state, target)
}

/// Once an operation ended: after `review.write` ran, the report takes the place of the buttons; when it was refused,
/// the page reads its files again.
pub fn ended(state: &mut State, feature: Option<FeatureId>, has_run: bool) -> Vec<Effect> {
    if feature != Some(FeatureId(WRITE)) {
        return Vec::new();
    }
    let Some(page) = state.reviews.page.as_mut() else {
        return Vec::new();
    };
    let writing = page.writing.take();
    if has_run {
        page.report = writing;
        page.accepted.clear();
        page.folds.clear();
        page.file_folds.clear();
        Vec::new()
    } else {
        read_again(state)
    }
}
