//! The comparison of two commits in the inspector (`compare.in-place`): its files read on a worker, swapped, opened as
//! a page, or in the Diff page.

use std::path::PathBuf;
use std::sync::Arc;

use git::read::compare::FilesRequest;
use model::compare::{
    CompareSide, ComparisonPart, ComparisonTarget, InPlaceSides, NamedCommit, Side, SideInput, follow_sides,
    initial_mode,
};
use model::state::FileChange;
use views::compare::{CompareIntent, InPlaceRead, InPlaceState, in_place_diff_target};

use crate::persisted::same_folder;
use crate::store::message::{Effect, JobId, JobKind};
use crate::store::state::State;

/// The comparison in the inspector.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InPlace {
    /// The folder of its repository.
    pub folder: PathBuf,
    /// The two commits, from and to.
    pub sides: InPlaceSides,
    /// Its files, once read; none while they are read.
    pub files: Option<Result<Vec<FileChange>, ()>>,
    /// The read running.
    pub job: Option<JobId>,
}

/// Shows the comparison of two commits in the inspector, in place of the card of the row selected, and reads its
/// files.
pub fn show(state: &mut State, folder: PathBuf, sides: InPlaceSides) -> Vec<Effect> {
    let mut effects = cancel(state);
    state.compares.in_place = Some(InPlace {
        folder,
        sides,
        files: None,
        job: None,
    });
    effects.extend(read(state));
    effects
}

/// Cancels the read of the comparison in the inspector, when one runs.
fn cancel(state: &mut State) -> Vec<Effect> {
    let running = state
        .compares
        .in_place
        .as_mut()
        .and_then(|in_place| in_place.job.take());
    running
        .map(|job| {
            let _cancelled = state.jobs.remove(&job);
            Effect::Cancel(job)
        })
        .into_iter()
        .collect()
}

/// Reads the files of the comparison in the inspector.
fn read(state: &mut State) -> Option<Effect> {
    let in_place = state.compares.in_place.as_ref()?;
    let kind = JobKind::ReadInPlace {
        folder: in_place.folder.clone(),
        request: FilesRequest {
            from: in_place.sides.from.commit.as_str().to_owned(),
            to: in_place.sides.to.commit.as_str().to_owned(),
        },
    };
    let job = crate::store::workspace::start_job(state, kind);
    if let Some(in_place) = state.compares.in_place.as_mut() {
        in_place.job = Some(job.id);
    }
    Some(Effect::Start(job))
}

/// Takes the files read; the read of a comparison closed or replaced meanwhile is dropped.
pub fn in_place_read(state: &mut State, job: JobId, result: Result<Vec<FileChange>, String>) -> Vec<Effect> {
    let _finished = state.jobs.remove(&job);
    let Some(in_place) = state
        .compares
        .in_place
        .as_mut()
        .filter(|in_place| in_place.job == Some(job))
    else {
        return Vec::new();
    };
    in_place.job = None;
    in_place.files = Some(result.map_err(|error| state.last_error = Some(error)));
    Vec::new()
}

/// Something asked of the comparison in the inspector.
pub fn reduce(state: &mut State, intent: &CompareIntent) -> Vec<Effect> {
    match intent {
        CompareIntent::CloseInPlace => {
            let effects = cancel(state);
            state.compares.in_place = None;
            effects
        }
        CompareIntent::SwapInPlace => {
            let mut effects = cancel(state);
            if let Some(in_place) = state.compares.in_place.as_mut() {
                let sides = &mut in_place.sides;
                std::mem::swap(&mut sides.from, &mut sides.to);
                in_place.files = None;
            }
            effects.extend(read(state));
            effects
        }
        CompareIntent::InPlaceAsPage => as_page(state),
        CompareIntent::InPlaceFile(index) => open_files(state, Some(*index)),
        CompareIntent::InPlaceAll => open_files(state, None),
        _ => Vec::new(),
    }
}

/// **Open as a page**: the comparison page of the same two sides, named as in the inspector, in the repository shown
/// when it is the one of the comparison.
fn as_page(state: &mut State) -> Vec<Effect> {
    let Some(in_place) = &state.compares.in_place else {
        return Vec::new();
    };
    let Some(read) = state
        .shown_read()
        .filter(|read| same_folder(&read.folder, &in_place.folder))
    else {
        return Vec::new();
    };
    let side = |named: &NamedCommit| -> CompareSide {
        let is_named = named.name != named.commit.short();
        follow_sides(&SideInput {
            name: &named.name,
            chosen: &Side::Commit(named.commit.clone()),
            is_followed: Some(is_named),
            resolved: Some(&named.commit),
        })
    };
    let (first, second) = (side(&in_place.sides.from), side(&in_place.sides.to));
    let mode = initial_mode(&(first.clone(), second.clone()));
    let target = ComparisonTarget {
        first,
        second,
        mode,
        path: None,
        merge_base: None,
        part: ComparisonPart::Whole,
    };
    let repository = (in_place.folder.clone(), Arc::clone(&read.state), read.generation);
    super::open_page(state, repository, target)
}

/// A file of the comparison in the inspector, or all of them, in one Diff page from the first commit to the second.
fn open_files(state: &mut State, index: Option<usize>) -> Vec<Effect> {
    let Some(InPlace {
        sides,
        files: Some(Ok(files)),
        ..
    }) = &state.compares.in_place
    else {
        return Vec::new();
    };
    let files = match index {
        Some(index) => files.get(index..=index).unwrap_or_default(),
        None => files,
    };
    if files.is_empty() {
        return Vec::new();
    }
    let target = in_place_diff_target(sides, files);
    crate::store::diff_page::beside::open(state, target)
}

/// What the inspector shows of the comparison in place of the card, while it is shown.
#[must_use]
pub fn in_place_state(state: &State) -> Option<InPlaceState<'_>> {
    let in_place = state.compares.in_place.as_ref()?;
    Some(InPlaceState {
        sides: &in_place.sides,
        read: match &in_place.files {
            None => InPlaceRead::Loading,
            Some(Ok(files)) => InPlaceRead::Read(files),
            Some(Err(())) => InPlaceRead::Failed,
        },
        marked: crate::store::diff_page::beside::shown_file(state).map(|shown| shown.path),
    })
}
